use super::*;
use crate::check::{
    CheckedBinaryOperator, CheckedCallArgument, CheckedCallTarget, CheckedExprId, CheckedExprKind,
    CheckedExprUseId, CheckedInitializerCoercion, CheckedLocalId, CheckedPathRoot,
    CheckedProjection, CheckedUnaryOperator, ExprTypeEnv,
};
use crate::lower::ExternFnId;
use crate::unqualified_name;

#[cfg(test)]
use std::cell::Cell;

pub(in crate::codegen) trait BindingEnvironment {
    fn get(&self, name: &str) -> Option<&Binding>;

    fn visit(&self, visitor: &mut dyn FnMut(&str, &Binding));

    fn binding_with_prefix(&self, prefix: &str) -> Option<&Binding>;

    fn snapshot(&self) -> HashMap<String, Binding> {
        #[cfg(test)]
        record_binding_env_full_clone();
        let mut snapshot = HashMap::new();
        self.visit(&mut |name, binding| {
            snapshot.insert(name.to_owned(), binding.clone());
        });
        snapshot
    }

    fn contains_key(&self, name: &str) -> bool {
        self.get(name).is_some()
    }

    fn component_context(&self) -> Option<(&str, &Binding)> {
        None
    }
}

pub(in crate::codegen) struct ScopedBindingEnv<'a> {
    base: &'a dyn BindingEnvironment,
    entries: Vec<(String, Binding)>,
}

impl<'a> ScopedBindingEnv<'a> {
    pub(in crate::codegen) fn new(base: &'a dyn BindingEnvironment) -> Self {
        #[cfg(test)]
        record_binding_env_overlay();
        Self {
            base,
            entries: Vec::new(),
        }
    }

    pub(in crate::codegen) fn insert(&mut self, name: String, binding: Binding) {
        if let Some((_, current)) = self.entries.iter_mut().find(|(key, _)| *key == name) {
            *current = binding;
        } else {
            self.entries.push((name, binding));
        }
    }
}

impl BindingEnvironment for ScopedBindingEnv<'_> {
    fn get(&self, name: &str) -> Option<&Binding> {
        self.entries
            .iter()
            .rev()
            .find_map(|(key, binding)| (key == name).then_some(binding))
            .or_else(|| self.base.get(name))
    }

    fn visit(&self, visitor: &mut dyn FnMut(&str, &Binding)) {
        self.base.visit(visitor);
        for (name, binding) in &self.entries {
            visitor(name, binding);
        }
    }

    fn binding_with_prefix(&self, prefix: &str) -> Option<&Binding> {
        self.entries
            .iter()
            .find_map(|(name, binding)| name.starts_with(prefix).then_some(binding))
            .or_else(|| self.base.binding_with_prefix(prefix))
    }

    fn component_context(&self) -> Option<(&str, &Binding)> {
        binding::component_context(self)
    }
}

impl BindingEnvironment for HashMap<String, Binding> {
    fn get(&self, name: &str) -> Option<&Binding> {
        HashMap::get(self, name)
    }

    fn visit(&self, visitor: &mut dyn FnMut(&str, &Binding)) {
        for (name, binding) in self {
            visitor(name, binding);
        }
    }

    fn binding_with_prefix(&self, prefix: &str) -> Option<&Binding> {
        self.iter()
            .find_map(|(name, binding)| name.starts_with(prefix).then_some(binding))
    }

    fn component_context(&self) -> Option<(&str, &Binding)> {
        component_context(self)
    }
}

pub(in crate::codegen) struct LayeredBindingEnv<'a> {
    base: &'a dyn BindingEnvironment,
    name: &'a str,
    binding: Binding,
}

impl<'a> LayeredBindingEnv<'a> {
    pub(in crate::codegen) fn new(
        base: &'a dyn BindingEnvironment,
        name: &'a str,
        binding: Binding,
    ) -> Self {
        #[cfg(test)]
        record_binding_env_overlay();
        Self {
            base,
            name,
            binding,
        }
    }
}

impl BindingEnvironment for LayeredBindingEnv<'_> {
    fn get(&self, name: &str) -> Option<&Binding> {
        if name == self.name {
            Some(&self.binding)
        } else {
            self.base.get(name)
        }
    }

    fn visit(&self, visitor: &mut dyn FnMut(&str, &Binding)) {
        self.base.visit(visitor);
        visitor(self.name, &self.binding);
    }

    fn binding_with_prefix(&self, prefix: &str) -> Option<&Binding> {
        self.name
            .starts_with(prefix)
            .then_some(&self.binding)
            .or_else(|| self.base.binding_with_prefix(prefix))
    }

    fn component_context(&self) -> Option<(&str, &Binding)> {
        self.base.component_context()
    }
}

struct BindingTypeEnv<'a>(&'a dyn BindingEnvironment);

impl ExprTypeEnv for BindingTypeEnv<'_> {
    fn get_type(&self, name: &str) -> Option<&Type> {
        self.0.get(name).map(|binding| &binding.ty)
    }

    fn visit_types(&self, visitor: &mut dyn FnMut(&str, &Type)) {
        self.0
            .visit(&mut |name, binding| visitor(name, &binding.ty));
    }

    fn type_with_prefix(&self, prefix: &str) -> Option<&Type> {
        self.0
            .binding_with_prefix(prefix)
            .map(|binding| &binding.ty)
    }
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(in crate::codegen) struct BindingEnvMetrics {
    pub(in crate::codegen) overlays: usize,
    pub(in crate::codegen) overlay_binding_allocations: usize,
    pub(in crate::codegen) binding_clone_allocations: usize,
    pub(in crate::codegen) scope_env_full_clones: usize,
}

#[cfg(test)]
thread_local! {
    static BINDING_ENV_METRICS: Cell<BindingEnvMetrics> = const { Cell::new(BindingEnvMetrics {
        overlays: 0,
        overlay_binding_allocations: 0,
        binding_clone_allocations: 0,
        scope_env_full_clones: 0,
    }) };
}

#[cfg(test)]
fn record_binding_clone() {
    BINDING_ENV_METRICS.with(|metrics| {
        let mut value = metrics.get();
        value.binding_clone_allocations += 1;
        metrics.set(value);
    });
}

#[cfg(test)]
fn record_binding_env_overlay() {
    BINDING_ENV_METRICS.with(|metrics| {
        let mut value = metrics.get();
        value.overlays += 1;
        value.overlay_binding_allocations += 1;
        metrics.set(value);
    });
}

#[cfg(test)]
fn record_binding_env_full_clone() {
    BINDING_ENV_METRICS.with(|metrics| {
        let mut value = metrics.get();
        value.scope_env_full_clones += 1;
        metrics.set(value);
    });
}

#[cfg(test)]
pub(in crate::codegen) fn reset_binding_env_metrics() {
    BINDING_ENV_METRICS.set(BindingEnvMetrics::default());
}

#[cfg(test)]
pub(in crate::codegen) fn binding_env_metrics() -> BindingEnvMetrics {
    BINDING_ENV_METRICS.get()
}

#[derive(Clone, Copy)]
enum ExprNode {
    Ast(u32),
    Checked(CheckedExprId),
}

enum ExprNodeKind<'a> {
    Bool(bool),
    I64(i64),
    F64(f64),
    Str(&'a str),
    Bytes(&'a [u8]),
    List(Vec<ExprNode>),
    None,
    SlotProvided(crate::hir::ComponentSlotId),
    Path(ExprPath<'a>),
    Call {
        target: ExprCallTarget<'a>,
        arguments: ExprArguments,
    },
    Unary {
        op: UnaryOp,
        value: ExprNode,
    },
    Binary {
        left: ExprNode,
        op: BinaryOp,
        right: ExprNode,
    },
}

enum ExprPath<'a> {
    Ast(&'a [String]),
    Checked {
        root: &'a CheckedPathRoot,
        projections: &'a [CheckedProjection],
    },
}

#[derive(Clone, Copy)]
enum ExprCallTarget<'a> {
    Ast(&'a str),
    Builtin(&'a str),
    Extern(ExternFnId),
    EnumVariant(crate::hir::EnumVariantId),
}

#[derive(Clone, Copy)]
enum ExprArgument {
    Value(ExprNode),
    Binding(CheckedLocalId),
}

struct ExprArguments(Vec<ExprArgument>);

impl ExprArguments {
    fn value(&self, index: usize) -> Result<ExprNode, Error> {
        match self.0.get(index) {
            Some(ExprArgument::Value(value)) => Ok(*value),
            Some(ExprArgument::Binding(_)) => Err(Error::new(
                "E196",
                &Span::line(1),
                "checked expression binding used as a value",
            )),
            None => Err(Error::new(
                "E196",
                &Span::line(1),
                "checked expression argument is missing",
            )),
        }
    }

    fn binding<'a>(
        &self,
        index: usize,
        context: &'a ExprEmission<'a>,
    ) -> Result<(&'a str, Option<CheckedLocalId>), Error> {
        match self.0.get(index) {
            Some(ExprArgument::Binding(id)) => Ok((
                &context
                    .program
                    .expect("checked binding has a lowered program")
                    .checked_facts()
                    .local(*id)
                    .name,
                Some(*id),
            )),
            Some(ExprArgument::Value(node))
                if context
                    .ast_expr(*node)
                    .is_some_and(|expr| matches!(expr, Expr::Path(path) if path.len() == 1)) =>
            {
                let Some(Expr::Path(path)) = context.ast_expr(*node) else {
                    unreachable!("guard verified the AST path")
                };
                Ok((&path[0], None))
            }
            _ => Err(Error::new(
                "E196",
                &Span::line(1),
                "animation projection has no normalized binding",
            )),
        }
    }

    fn get(&self, index: usize) -> Option<ExprArgument> {
        self.0.get(index).copied()
    }

    fn values(&self) -> Result<Vec<ExprNode>, Error> {
        self.0
            .iter()
            .map(|argument| match argument {
                ExprArgument::Value(value) => Ok(*value),
                ExprArgument::Binding(_) => Err(Error::new(
                    "E196",
                    &Span::line(1),
                    "checked binding reached ordinary argument emission",
                )),
            })
            .collect()
    }
}

struct ExprEmission<'a> {
    document: &'a Document,
    program: Option<&'a LoweredProgram>,
    ast_nodes: Vec<&'a Expr>,
    ast_ids: HashMap<usize, u32>,
}

impl<'a> ExprEmission<'a> {
    fn for_ast(document: &'a Document, root: &'a Expr) -> Self {
        fn collect<'a>(expr: &'a Expr, nodes: &mut Vec<&'a Expr>, ids: &mut HashMap<usize, u32>) {
            let key = std::ptr::from_ref(expr) as usize;
            if ids.contains_key(&key) {
                return;
            }
            let id = u32::try_from(nodes.len()).expect("expression arena exceeds u32");
            ids.insert(key, id);
            nodes.push(expr);
            match expr {
                Expr::List(values) | Expr::Call { args: values, .. } => {
                    for value in values {
                        collect(value, nodes, ids);
                    }
                }
                Expr::Unary { value, .. } => collect(value, nodes, ids),
                Expr::Binary { left, right, .. } => {
                    collect(left, nodes, ids);
                    collect(right, nodes, ids);
                }
                _ => {}
            }
        }

        let mut ast_nodes = Vec::new();
        let mut ast_ids = HashMap::new();
        collect(root, &mut ast_nodes, &mut ast_ids);
        Self {
            document,
            program: None,
            ast_nodes,
            ast_ids,
        }
    }

    fn for_checked(program: &'a LoweredProgram) -> Self {
        Self {
            document: program.document(),
            program: Some(program),
            ast_nodes: Vec::new(),
            ast_ids: HashMap::new(),
        }
    }

    fn ast_node(&self, expr: &Expr) -> ExprNode {
        let key = std::ptr::from_ref(expr) as usize;
        ExprNode::Ast(self.ast_ids[&key])
    }

    fn ast_expr(&self, node: ExprNode) -> Option<&'a Expr> {
        match node {
            ExprNode::Ast(id) => self.ast_nodes.get(id as usize).copied(),
            ExprNode::Checked(_) => None,
        }
    }

    fn kind(&self, node: ExprNode) -> ExprNodeKind<'a> {
        match node {
            ExprNode::Ast(id) => match self.ast_nodes[id as usize] {
                Expr::Bool(value) => ExprNodeKind::Bool(*value),
                Expr::I64(value) => ExprNodeKind::I64(*value),
                Expr::F64(value) => ExprNodeKind::F64(*value),
                Expr::Str(value) => ExprNodeKind::Str(value),
                Expr::Bytes(values) => ExprNodeKind::Bytes(values),
                Expr::EmptyList => ExprNodeKind::List(Vec::new()),
                Expr::List(values) => {
                    ExprNodeKind::List(values.iter().map(|value| self.ast_node(value)).collect())
                }
                Expr::None => ExprNodeKind::None,
                Expr::Path(path) => ExprNodeKind::Path(ExprPath::Ast(path)),
                Expr::Call { name, args } => ExprNodeKind::Call {
                    target: ExprCallTarget::Ast(name),
                    arguments: ExprArguments(
                        args.iter()
                            .map(|value| ExprArgument::Value(self.ast_node(value)))
                            .collect(),
                    ),
                },
                Expr::Unary { op, value } => ExprNodeKind::Unary {
                    op: *op,
                    value: self.ast_node(value),
                },
                Expr::Binary { left, op, right } => ExprNodeKind::Binary {
                    left: self.ast_node(left),
                    op: *op,
                    right: self.ast_node(right),
                },
            },
            ExprNode::Checked(id) => {
                let expression = self
                    .program
                    .expect("checked expression has a lowered program")
                    .checked_facts()
                    .expression(id);
                match &expression.kind {
                    CheckedExprKind::Bool(value) => ExprNodeKind::Bool(*value),
                    CheckedExprKind::I64(value) => ExprNodeKind::I64(*value),
                    CheckedExprKind::F64(value) => ExprNodeKind::F64(*value),
                    CheckedExprKind::Str(value) => ExprNodeKind::Str(value),
                    CheckedExprKind::Bytes(values) => ExprNodeKind::Bytes(values),
                    CheckedExprKind::List(values) => {
                        ExprNodeKind::List(values.iter().copied().map(ExprNode::Checked).collect())
                    }
                    CheckedExprKind::None => ExprNodeKind::None,
                    CheckedExprKind::SlotProvided(slot) => ExprNodeKind::SlotProvided(*slot),
                    CheckedExprKind::Path { root, projections } => {
                        ExprNodeKind::Path(ExprPath::Checked { root, projections })
                    }
                    CheckedExprKind::Call { target, arguments } => ExprNodeKind::Call {
                        target: match target {
                            CheckedCallTarget::Builtin(id) => ExprCallTarget::Builtin(
                                self.program
                                    .expect("checked builtin has a lowered program")
                                    .checked_facts()
                                    .builtin(*id),
                            ),
                            CheckedCallTarget::Extern(reference) => {
                                ExprCallTarget::Extern(reference.id)
                            }
                            CheckedCallTarget::EnumVariant(id) => ExprCallTarget::EnumVariant(*id),
                        },
                        arguments: ExprArguments(
                            arguments
                                .iter()
                                .map(|argument| match argument {
                                    CheckedCallArgument::Value(id) => {
                                        ExprArgument::Value(ExprNode::Checked(*id))
                                    }
                                    CheckedCallArgument::Binding(id) => ExprArgument::Binding(*id),
                                })
                                .collect(),
                        ),
                    },
                    CheckedExprKind::Unary { operator, value } => ExprNodeKind::Unary {
                        op: match operator {
                            CheckedUnaryOperator::BooleanNot => UnaryOp::Not,
                            CheckedUnaryOperator::NumericNegation(_) => UnaryOp::Neg,
                        },
                        value: ExprNode::Checked(*value),
                    },
                    CheckedExprKind::Binary {
                        operator,
                        left,
                        right,
                    } => ExprNodeKind::Binary {
                        left: ExprNode::Checked(*left),
                        op: match operator {
                            CheckedBinaryOperator::Boolean(op)
                            | CheckedBinaryOperator::Equality { op, .. }
                            | CheckedBinaryOperator::Ordering { op, .. }
                            | CheckedBinaryOperator::Arithmetic { op, .. } => *op,
                        },
                        right: ExprNode::Checked(*right),
                    },
                }
            }
        }
    }

    fn ty(&self, node: ExprNode, env: &dyn BindingEnvironment) -> Result<Type, Error> {
        match node {
            ExprNode::Ast(id) => expr_type(
                self.ast_nodes[id as usize],
                &BindingTypeEnv(env),
                self.document,
                &Span::line(1),
            ),
            ExprNode::Checked(id) => Ok(self
                .program
                .expect("checked expression has a lowered program")
                .checked_facts()
                .expression(id)
                .ty
                .clone()),
        }
    }

    fn literal_i64(&self, node: ExprNode) -> Option<i64> {
        match self.kind(node) {
            ExprNodeKind::I64(value) => Some(value),
            _ => None,
        }
    }

    fn literal_str(&self, node: ExprNode) -> Option<&'a str> {
        match self.kind(node) {
            ExprNodeKind::Str(value) => Some(value),
            _ => None,
        }
    }

    fn is_none(&self, node: ExprNode) -> bool {
        matches!(self.kind(node), ExprNodeKind::None)
    }
}

pub(in crate::codegen) fn expr_list_code(
    values: &[Expr],
    env: &dyn BindingEnvironment,
    document: &Document,
) -> Result<String, Error> {
    Ok(values
        .iter()
        .map(|value| expr_code(value, env, document, ValueMode::Owned))
        .collect::<Result<Vec<_>, _>>()?
        .join(", "))
}

pub(in crate::codegen) fn expr_code(
    expr: &Expr,
    env: &dyn BindingEnvironment,
    document: &Document,
    mode: ValueMode,
) -> Result<String, Error> {
    let context = ExprEmission::for_ast(document, expr);
    expr_node_code(context.ast_node(expr), env, &context, mode)
}

fn expr_node_code(
    expr: ExprNode,
    env: &dyn BindingEnvironment,
    context: &ExprEmission<'_>,
    mode: ValueMode,
) -> Result<String, Error> {
    let document = context.document;
    Ok(match context.kind(expr) {
        ExprNodeKind::Bool(value) => value.to_string(),
        ExprNodeKind::I64(value) => value.to_string(),
        ExprNodeKind::F64(value) => rust_f64(value),
        ExprNodeKind::Str(value) => match mode {
            ValueMode::Owned => format!("{}.to_owned()", rust_string(value)),
            ValueMode::Borrowed => rust_string(value),
        },
        ExprNodeKind::Bytes(values) => format!(
            "::std::vec![{}]",
            values
                .iter()
                .map(|value| format!("0x{value:02x}u8"))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        ExprNodeKind::List(values) if values.is_empty() => "::std::vec::Vec::new()".into(),
        ExprNodeKind::List(values) => format!(
            "::std::vec![{}]",
            expr_node_list_code(&values, env, context)?
        ),
        ExprNodeKind::None => "::std::option::Option::None".into(),
        ExprNodeKind::SlotProvided(slot) => {
            let slot = context
                .program
                .expect("checked slot expression has a lowered program")
                .component_slot_name(slot)?;
            env.contains_key(&format!("\0slot-provided:{slot}"))
                .to_string()
        }
        ExprNodeKind::Path(ExprPath::Ast(path)) => {
            if let [contract, palette] = path
                && document
                    .theme_contract
                    .as_ref()
                    .is_some_and(|item| item.name == *contract)
                && document.palettes.iter().any(|item| item.name == *palette)
            {
                return Ok(format!(
                    "{}::{}",
                    generated_named_rust(contract),
                    pascal(palette)
                ));
            }
            if let [enum_name, variant_name] = path
                && document.enums.iter().any(|item| {
                    item.name == *enum_name
                        && item.variants.iter().any(|variant| {
                            variant.name == *variant_name && variant.payload.is_none()
                        })
                })
            {
                return Ok(format!(
                    "{}::{}",
                    generated_named_rust(enum_name),
                    pascal(variant_name)
                ));
            }
            let binding = env.get(&path[0]).ok_or_else(|| {
                Error::new(
                    "E150",
                    &Span::line(1),
                    format!("unknown value `{}`", path[0]),
                )
            })?;
            let mut code = binding.code.clone();
            let mut ty = binding.ty.clone();
            let mut owned_projection = false;
            for field in &path[1..] {
                if let Some((projection, projected_ty)) = native_field_projection(&ty, field, &code)
                {
                    code = projection;
                    ty = projected_ty;
                    owned_projection = true;
                    continue;
                }
                if let Type::Option(inner) = &ty
                    && **inner == Type::WidgetTarget
                {
                    code = format!("({code}).as_ref().map(|value| value.{field}.clone())");
                    ty = Type::Option(Box::new(
                        widget_target_field_type(field).unwrap_or(Type::Unknown),
                    ));
                    owned_projection = true;
                    continue;
                }
                write!(code, ".{field}").unwrap();
                if let Type::Named(name) = &ty {
                    ty = document
                        .structs
                        .iter()
                        .find(|item| item.name == *name)
                        .and_then(|item| item.fields.iter().find(|(name, _)| name == field))
                        .map(|(_, ty)| ty.clone())
                        .unwrap_or(Type::Unknown);
                } else if ty == Type::WidgetTarget {
                    ty = widget_target_field_type(field).unwrap_or(Type::Unknown);
                } else if let Some(field_ty) = native_field_type(&ty, field) {
                    ty = field_ty;
                }
            }
            let clone_unnecessary = matches!(
                ty,
                Type::Bool
                    | Type::I64
                    | Type::F64
                    | Type::PhysicalKey
                    | Type::KeyLocation
                    | Type::KeyModifiers
                    | Type::Pixels
                    | Type::Padding
                    | Type::Degrees
                    | Type::Radians
                    | Type::Rotation
                    | Type::ContentFit
                    | Type::Color
                    | Type::Background
                    | Type::Gradient
                    | Type::LinearGradient
                    | Type::ColorStop
                    | Type::Font
                    | Type::FontFamily
                    | Type::FontWeight
                    | Type::FontStretch
                    | Type::FontStyle
                    | Type::ThemeMode
                    | Type::TextAlignment
                    | Type::TextShaping
                    | Type::TextWrapping
                    | Type::TextLineHeight
                    | Type::MouseInteraction
                    | Type::ScrollDelta
                    | Type::EventStatus
                    | Type::Length
                    | Type::Alignment
                    | Type::HorizontalAlignment
                    | Type::VerticalAlignment
                    | Type::Border
                    | Type::Radius
                    | Type::Shadow
                    | Type::Point
                    | Type::PointU32
                    | Type::Vector
                    | Type::Size
                    | Type::SizeU32
                    | Type::Rectangle
                    | Type::RectangleU32
                    | Type::Transformation
                    | Type::MouseButton
                    | Type::MouseCursor
                    | Type::MouseClick
                    | Type::TouchFinger
                    | Type::WindowId
                    | Type::WindowPosition
                    | Type::RedrawRequest
                    | Type::WindowDirection
                    | Type::WindowLevel
                    | Type::WindowMode
                    | Type::WindowAttention
                    | Type::Unit
            ) || (binding.local && path.len() == 1)
                || owned_projection;
            if matches!(mode, ValueMode::Owned) && !clone_unnecessary {
                if ty == Type::Str {
                    code.push_str(".to_owned()");
                } else {
                    code.push_str(".clone()");
                }
            }
            code
        }
        ExprNodeKind::Path(ExprPath::Checked { root, projections }) => {
            checked_path_code(root, projections, env, context, mode)?
        }
        ExprNodeKind::Call { target, arguments } => {
            let args = arguments;
            let name = match target {
                ExprCallTarget::EnumVariant(id) => {
                    let program = context.program.expect("checked enum has a lowered program");
                    let variant = program.declarations().enum_variant_decl(id);
                    let owner = program.declarations().enum_decl(id.owner);
                    return Ok(format!(
                        "{}::{}({})",
                        owner.rust_name,
                        pascal(&variant.name),
                        expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?
                    ));
                }
                ExprCallTarget::Extern(id) => {
                    let function = context
                        .program
                        .expect("checked extern has a lowered program")
                        .extern_function(id);
                    let args = expr_node_list_code(&args.values()?, env, context)?;
                    return Ok(format!("{}({args})", function.rust_path));
                }
                ExprCallTarget::Builtin(name) => name,
                ExprCallTarget::Ast(name) => name,
            };
            if unqualified_name(name) == "provided" {
                let value = args.value(0)?;
                let Some(Expr::Path(path)) = context.ast_expr(value) else {
                    return Err(Error::new(
                        "E196",
                        &Span::line(1),
                        "provided slot was not preserved as an AST view expression",
                    ));
                };
                let [slot] = path.as_slice() else {
                    return Err(Error::new("E196", &Span::line(1), "invalid provided slot"));
                };
                let slot = unqualified_name(slot);
                return Ok(env
                    .contains_key(&format!("\0slot-provided:{slot}"))
                    .to_string());
            }
            if matches!(target, ExprCallTarget::Ast(_))
                && let Some((enum_name, variant_name)) = name.split_once('.')
                && document.enums.iter().any(|item| {
                    item.name == enum_name
                        && item.variants.iter().any(|variant| {
                            variant.name == variant_name && variant.payload.is_some()
                        })
                })
            {
                return Ok(format!(
                    "{}::{}({})",
                    generated_named_rust(enum_name),
                    pascal(variant_name),
                    expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?
                ));
            }
            if matches!(target, ExprCallTarget::Ast(_))
                && let Some(function) = find_extern_function(document, name, ExternKind::Sync)
            {
                let args = expr_node_list_code(&args.values()?, env, context)?;
                return Ok(format!("{}({args})", function.rust_path));
            }
            let name = unqualified_name(name);
            if let Some(code) = expr_builtin_group_1(name, &args, env, context, mode)? {
                code
            } else if let Some(code) = expr_builtin_group_2(name, &args, env, context, mode)? {
                code
            } else if let Some(code) = expr_builtin_group_3(name, &args, env, context, mode)? {
                code
            } else if let Some(code) = expr_builtin_group_4(name, &args, env, context, mode)? {
                code
            } else if let Some(code) = expr_builtin_group_5(name, &args, env, context, mode)? {
                code
            } else {
                expr_builtin_group_6(name, &args, env, context, mode)?
                    .expect("checked builtin or declared extern call")
            }
        }
        ExprNodeKind::Unary { op, value } => format!(
            "({}{})",
            match op {
                UnaryOp::Not => "!",
                UnaryOp::Neg => "-",
            },
            expr_node_code(value, env, context, ValueMode::Owned)?
        ),
        ExprNodeKind::Binary { left, op, right } => {
            let mode = if matches!(
                &op,
                BinaryOp::Eq
                    | BinaryOp::NotEq
                    | BinaryOp::Lt
                    | BinaryOp::LtEq
                    | BinaryOp::Gt
                    | BinaryOp::GtEq
            ) {
                ValueMode::Borrowed
            } else {
                ValueMode::Owned
            };
            let left_ty = context.ty(left, env)?;
            let right_ty = context.ty(right, env)?;
            let left = expr_node_code(left, env, context, mode)?;
            let right = expr_node_code(right, env, context, mode)?;
            let left = if left_ty == Type::F64 && right_ty == Type::Radians && op == BinaryOp::Mul {
                format!("({left}) as f32")
            } else {
                left
            };
            let right = if right_ty == Type::F64
                && matches!(
                    left_ty,
                    Type::Pixels
                        | Type::Degrees
                        | Type::Radians
                        | Type::Radius
                        | Type::Vector
                        | Type::Size
                        | Type::Rectangle
                ) {
                format!("({right}) as f32")
            } else {
                right
            };
            format!(
                "({} {} {})",
                left,
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
                right
            )
        }
    })
}

fn expr_builtin_group_1(
    name: &str,
    args: &ExprArguments,
    env: &dyn BindingEnvironment,
    context: &ExprEmission<'_>,
    _mode: ValueMode,
) -> Result<Option<String>, Error> {
    Ok(Some(match name {
        "color.default" => "::iced::Color::default()".into(),
        "color.black" => "::iced::Color::BLACK".into(),
        "color.white" => "::iced::Color::WHITE".into(),
        "color.transparent" => "::iced::Color::TRANSPARENT".into(),
        "color.rgb" => format!(
            "::iced::Color::from_rgb({}, {}, {})",
            node_unit_f32_code(args.value(0)?, env, context)?,
            node_unit_f32_code(args.value(1)?, env, context)?,
            node_unit_f32_code(args.value(2)?, env, context)?
        ),
        "color.rgba" => format!(
            "::iced::Color::from_rgba({}, {}, {}, {})",
            node_unit_f32_code(args.value(0)?, env, context)?,
            node_unit_f32_code(args.value(1)?, env, context)?,
            node_unit_f32_code(args.value(2)?, env, context)?,
            node_unit_f32_code(args.value(3)?, env, context)?
        ),
        "color.rgb8" => {
            let r = context
                .literal_i64(args.value(0)?)
                .expect("checked u8 channel");
            let g = context
                .literal_i64(args.value(1)?)
                .expect("checked u8 channel");
            let b = context
                .literal_i64(args.value(2)?)
                .expect("checked u8 channel");
            format!("::iced::Color::from_rgb8({r}u8, {g}u8, {b}u8)")
        }
        "color.rgba8" => {
            let r = context
                .literal_i64(args.value(0)?)
                .expect("checked u8 channel");
            let g = context
                .literal_i64(args.value(1)?)
                .expect("checked u8 channel");
            let b = context
                .literal_i64(args.value(2)?)
                .expect("checked u8 channel");
            format!(
                "::iced::Color::from_rgba8({r}u8, {g}u8, {b}u8, {})",
                node_unit_f32_code(args.value(3)?, env, context)?
            )
        }
        "color.try_rgb8" | "color.try_rgba8" => {
            let red = expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?;
            let green = expr_node_code(args.value(1)?, env, context, ValueMode::Owned)?;
            let blue = expr_node_code(args.value(2)?, env, context, ValueMode::Owned)?;
            let constructor = if name == "color.try_rgb8" {
                "::iced::Color::from_rgb8(__red, __green, __blue)"
            } else {
                "::iced::Color::from_rgba8(__red, __green, __blue, __alpha as f32)"
            };
            if name == "color.try_rgb8" {
                format!(
                    "match (<u8>::try_from({red}), <u8>::try_from({green}), <u8>::try_from({blue})) {{ (::std::result::Result::Ok(__red), ::std::result::Result::Ok(__green), ::std::result::Result::Ok(__blue)) => ::std::option::Option::Some({constructor}), _ => ::std::option::Option::None }}"
                )
            } else {
                let alpha = expr_node_code(args.value(3)?, env, context, ValueMode::Owned)?;
                format!(
                    "{{ let __alpha = {alpha}; match (<u8>::try_from({red}), <u8>::try_from({green}), <u8>::try_from({blue})) {{ (::std::result::Result::Ok(__red), ::std::result::Result::Ok(__green), ::std::result::Result::Ok(__blue)) if (0.0..=1.0).contains(&__alpha) => ::std::option::Option::Some({constructor}), _ => ::std::option::Option::None }} }}"
                )
            }
        }
        "color.linear_rgba" => format!(
            "::iced::Color::from_linear_rgba({}, {}, {}, {})",
            node_unit_f32_code(args.value(0)?, env, context)?,
            node_unit_f32_code(args.value(1)?, env, context)?,
            node_unit_f32_code(args.value(2)?, env, context)?,
            node_unit_f32_code(args.value(3)?, env, context)?
        ),
        "color.from3" => format!(
            "::iced::Color::from([{}, {}, {}])",
            node_unit_f32_code(args.value(0)?, env, context)?,
            node_unit_f32_code(args.value(1)?, env, context)?,
            node_unit_f32_code(args.value(2)?, env, context)?
        ),
        "color.from4" => format!(
            "::iced::Color::from([{}, {}, {}, {}])",
            node_unit_f32_code(args.value(0)?, env, context)?,
            node_unit_f32_code(args.value(1)?, env, context)?,
            node_unit_f32_code(args.value(2)?, env, context)?,
            node_unit_f32_code(args.value(3)?, env, context)?
        ),
        "color.parse" => format!(
            "({}).parse::<::iced::Color>().ok()",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?
        ),
        "color.inverse" => format!(
            "({}).inverse()",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?
        ),
        "color.invert" => format!(
            "({}).inverse()",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?
        ),
        "color.scale_alpha" => format!(
            "({}).scale_alpha(({}) as f32)",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?,
            expr_node_code(args.value(1)?, env, context, ValueMode::Owned)?
        ),
        "color.luminance" => format!(
            "({}).relative_luminance() as f64",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?
        ),
        "color.contrast" => format!(
            "({}).relative_contrast({}) as f64",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?,
            expr_node_code(args.value(1)?, env, context, ValueMode::Owned)?
        ),
        "color.readable" => format!(
            "({}).is_readable_on({})",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?,
            expr_node_code(args.value(1)?, env, context, ValueMode::Owned)?
        ),
        "color_stop.default" => "::iced::gradient::ColorStop::default()".into(),
        "color_stop" => format!(
            "::iced::gradient::ColorStop {{ offset: ({}) as f32, color: {} }}",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?,
            expr_node_code(args.value(1)?, env, context, ValueMode::Owned)?
        ),
        "linear" => format!(
            "::iced::gradient::Linear::new({})",
            node_radians_value_code(args.value(0)?, env, context)?
        ),
        "linear.add_stop" => format!(
            "::ui_lang_runtime::add_gradient_stops({}, [::iced::gradient::ColorStop {{ offset: ({}) as f32, color: {} }}])",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?,
            expr_node_code(args.value(1)?, env, context, ValueMode::Owned)?,
            expr_node_code(args.value(2)?, env, context, ValueMode::Owned)?
        ),
        "linear.add_stops" => format!(
            "::ui_lang_runtime::add_gradient_stops({}, {})",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?,
            expr_node_code(args.value(1)?, env, context, ValueMode::Owned)?
        ),
        "linear.scale_alpha" => format!(
            "({}).scale_alpha(({}) as f32)",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?,
            expr_node_code(args.value(1)?, env, context, ValueMode::Owned)?
        ),
        "gradient.linear" => format!(
            "::iced::Gradient::Linear({})",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?
        ),
        "gradient.from_linear" => format!(
            "::iced::Gradient::from({})",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?
        ),
        "gradient.scale_alpha" => format!(
            "({}).scale_alpha(({}) as f32)",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?,
            expr_node_code(args.value(1)?, env, context, ValueMode::Owned)?
        ),
        "background.color" => format!(
            "::iced::Background::Color({})",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?
        ),
        "background.gradient" => format!(
            "::iced::Background::Gradient({})",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?
        ),
        "background.from_color" => format!(
            "::iced::Background::from({})",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?
        ),
        "background.from_gradient" => format!(
            "::iced::Background::from({})",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?
        ),
        "background.from_linear" => format!(
            "::iced::Background::from({})",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?
        ),
        "background.scale_alpha" => format!(
            "({}).scale_alpha(({}) as f32)",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?,
            expr_node_code(args.value(1)?, env, context, ValueMode::Owned)?
        ),
        _ => return Ok(None),
    }))
}
fn expr_builtin_group_2(
    name: &str,
    args: &ExprArguments,
    env: &dyn BindingEnvironment,
    context: &ExprEmission<'_>,
    _mode: ValueMode,
) -> Result<Option<String>, Error> {
    Ok(Some(match name {
        "font.default" => "::iced::Font::default()".into(),
        "font.sans" => "::iced::Font::DEFAULT".into(),
        "font.monospace" => "::iced::Font::MONOSPACE".into(),
        "font.with_name" => {
            let name = context
                .literal_str(args.value(0)?)
                .expect("checked font name literal");
            format!("::iced::Font::with_name({})", rust_string(name))
        }
        "font.new" => format!(
            "::iced::Font {{ family: {}, weight: {}, stretch: {}, style: {} }}",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?,
            expr_node_code(args.value(1)?, env, context, ValueMode::Owned)?,
            expr_node_code(args.value(2)?, env, context, ValueMode::Owned)?,
            expr_node_code(args.value(3)?, env, context, ValueMode::Owned)?
        ),
        "family.default" => "::iced::font::Family::default()".into(),
        "family.named" => {
            let name = context
                .literal_str(args.value(0)?)
                .expect("checked family name literal");
            format!("::iced::font::Family::Name({})", rust_string(name))
        }
        "family.serif" | "family.sans_serif" | "family.cursive" | "family.fantasy"
        | "family.monospace" => format!(
            "::iced::font::Family::{}",
            pascal(name.strip_prefix("family.").expect("checked prefix"))
        ),
        "weight.default" => "::iced::font::Weight::default()".into(),
        "weight.thin" | "weight.extra_light" | "weight.light" | "weight.normal"
        | "weight.medium" | "weight.semibold" | "weight.bold" | "weight.extra_bold"
        | "weight.black" => format!(
            "::iced::font::Weight::{}",
            pascal(name.strip_prefix("weight.").expect("checked prefix"))
        ),
        "stretch.default" => "::iced::font::Stretch::default()".into(),
        "stretch.ultra_condensed"
        | "stretch.extra_condensed"
        | "stretch.condensed"
        | "stretch.semi_condensed"
        | "stretch.normal"
        | "stretch.semi_expanded"
        | "stretch.expanded"
        | "stretch.extra_expanded"
        | "stretch.ultra_expanded" => format!(
            "::iced::font::Stretch::{}",
            pascal(name.strip_prefix("stretch.").expect("checked prefix"))
        ),
        "font_style.default" => "::iced::font::Style::default()".into(),
        "font_style.normal" | "font_style.italic" | "font_style.oblique" => format!(
            "::iced::font::Style::{}",
            pascal(name.strip_prefix("font_style.").expect("checked prefix"))
        ),
        "theme_mode.default" => "::iced::theme::Mode::default()".into(),
        "theme_mode.none" => "::iced::theme::Mode::None".into(),
        "theme_mode.light" => "::iced::theme::Mode::Light".into(),
        "theme_mode.dark" => "::iced::theme::Mode::Dark".into(),
        "text_alignment.default" => "::iced::widget::text::Alignment::default()".into(),
        "text_alignment.left"
        | "text_alignment.center"
        | "text_alignment.right"
        | "text_alignment.justified" => format!(
            "::iced::widget::text::Alignment::{}",
            pascal(
                name.strip_prefix("text_alignment.")
                    .expect("checked prefix")
            )
        ),
        "text_alignment.from_horizontal" | "text_alignment.from_alignment" => format!(
            "::iced::widget::text::Alignment::from({})",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?
        ),
        "horizontal.from_text_alignment" => format!(
            "::iced::alignment::Horizontal::from({})",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?
        ),
        "text_shaping.default" => "::iced::widget::text::Shaping::default()".into(),
        "text_shaping.auto" | "text_shaping.basic" | "text_shaping.advanced" => format!(
            "::iced::widget::text::Shaping::{}",
            pascal(name.strip_prefix("text_shaping.").expect("checked prefix"))
        ),
        "text_wrapping.default" => "::iced::widget::text::Wrapping::default()".into(),
        "text_wrapping.none"
        | "text_wrapping.word"
        | "text_wrapping.glyph"
        | "text_wrapping.word_or_glyph" => format!(
            "::iced::widget::text::Wrapping::{}",
            pascal(name.strip_prefix("text_wrapping.").expect("checked prefix"))
        ),
        "line_height.default" => "::iced::widget::text::LineHeight::default()".into(),
        "line_height.relative" => format!(
            "::iced::widget::text::LineHeight::Relative(({}) as f32)",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?
        ),
        "line_height.absolute" => format!(
            "::iced::widget::text::LineHeight::Absolute({})",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?
        ),
        "line_height.from_f64" => format!(
            "::iced::widget::text::LineHeight::from(({}) as f32)",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?
        ),
        "line_height.from_pixels" => format!(
            "::iced::widget::text::LineHeight::from({})",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?
        ),
        "line_height.to_absolute" => format!(
            "({}).to_absolute({})",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?,
            expr_node_code(args.value(1)?, env, context, ValueMode::Owned)?
        ),
        "screenshot.new" => format!(
            "::iced::window::Screenshot::new({}, {}, ({}) as f32)",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?,
            expr_node_code(args.value(1)?, env, context, ValueMode::Owned)?,
            expr_node_code(args.value(2)?, env, context, ValueMode::Owned)?
        ),
        "screenshot.crop" => format!(
            "::ui_lang_runtime::crop_screenshot(&({}), {}).ok()",
            expr_node_code(args.value(0)?, env, context, ValueMode::Borrowed)?,
            expr_node_code(args.value(1)?, env, context, ValueMode::Owned)?
        ),
        "screenshot.crop_error" => format!(
            "match ::ui_lang_runtime::crop_screenshot(&({}), {}) {{ ::std::result::Result::Ok(_) => ::std::option::Option::None, ::std::result::Result::Err(::iced::window::screenshot::CropError::Zero) => ::std::option::Option::Some(\"zero\".to_owned()), ::std::result::Result::Err(::iced::window::screenshot::CropError::OutOfBounds) => ::std::option::Option::Some(\"out-of-bounds\".to_owned()) }}",
            expr_node_code(args.value(0)?, env, context, ValueMode::Borrowed)?,
            expr_node_code(args.value(1)?, env, context, ValueMode::Owned)?
        ),
        "screenshot.crop_error_message" => format!(
            "::ui_lang_runtime::crop_screenshot(&({}), {}).err().map(|error| error.to_string())",
            expr_node_code(args.value(0)?, env, context, ValueMode::Borrowed)?,
            expr_node_code(args.value(1)?, env, context, ValueMode::Owned)?
        ),
        "screenshot.as_bytes" => format!(
            "::std::convert::AsRef::<[u8]>::as_ref(&({})).to_vec()",
            expr_node_code(args.value(0)?, env, context, ValueMode::Borrowed)?
        ),
        "screenshot.into_bytes" => format!(
            "({}).rgba.to_vec()",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?
        ),
        "interaction.default" => "::iced::mouse::Interaction::default()".into(),
        "interaction.none"
        | "interaction.hidden"
        | "interaction.idle"
        | "interaction.context_menu"
        | "interaction.help"
        | "interaction.pointer"
        | "interaction.progress"
        | "interaction.wait"
        | "interaction.cell"
        | "interaction.crosshair"
        | "interaction.text"
        | "interaction.alias"
        | "interaction.copy"
        | "interaction.move"
        | "interaction.no_drop"
        | "interaction.not_allowed"
        | "interaction.grab"
        | "interaction.grabbing"
        | "interaction.resize_horizontal"
        | "interaction.resize_vertical"
        | "interaction.resize_diagonal_up"
        | "interaction.resize_diagonal_down"
        | "interaction.resize_column"
        | "interaction.resize_row"
        | "interaction.all_scroll"
        | "interaction.zoom_in"
        | "interaction.zoom_out" => format!(
            "::iced::mouse::Interaction::{}",
            first_class_mouse_interaction_code(name)
        ),
        "scroll.lines" | "scroll.pixels" => format!(
            "::iced::mouse::ScrollDelta::{} {{ x: ({}) as f32, y: ({}) as f32 }}",
            if name == "scroll.lines" {
                "Lines"
            } else {
                "Pixels"
            },
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?,
            expr_node_code(args.value(1)?, env, context, ValueMode::Owned)?
        ),
        "event_status.ignored" => "::iced::event::Status::Ignored".into(),
        "event_status.captured" => "::iced::event::Status::Captured".into(),
        "event_status.merge" => format!(
            "({}).merge({})",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?,
            expr_node_code(args.value(1)?, env, context, ValueMode::Owned)?
        ),
        "redraw_request.next_frame" => "::iced::window::RedrawRequest::NextFrame".into(),
        "redraw_request.wait" => "::iced::window::RedrawRequest::Wait".into(),
        "redraw_request.at" => format!(
            "::iced::window::RedrawRequest::At({})",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?
        ),
        "window_id.unique" => "::iced::window::Id::unique()".into(),
        "window_direction.north"
        | "window_direction.south"
        | "window_direction.east"
        | "window_direction.west"
        | "window_direction.north_east"
        | "window_direction.north_west"
        | "window_direction.south_east"
        | "window_direction.south_west" => format!(
            "::iced::window::Direction::{}",
            pascal(
                name.strip_prefix("window_direction.")
                    .expect("checked prefix")
            )
        ),
        "window_level.default" => "::iced::window::Level::default()".into(),
        "window_level.normal" | "window_level.always_on_bottom" | "window_level.always_on_top" => {
            format!(
                "::iced::window::Level::{}",
                pascal(name.strip_prefix("window_level.").expect("checked prefix"))
            )
        }
        "window_mode.windowed" | "window_mode.fullscreen" | "window_mode.hidden" => {
            format!(
                "::iced::window::Mode::{}",
                pascal(name.strip_prefix("window_mode.").expect("checked prefix"))
            )
        }
        "window_attention.critical" | "window_attention.informational" => format!(
            "::iced::window::UserAttention::{}",
            pascal(
                name.strip_prefix("window_attention.")
                    .expect("checked prefix")
            )
        ),
        "window_position.default" => "::iced::window::Position::default()".into(),
        "window_position.centered" => "::iced::window::Position::Centered".into(),
        "window_position.specific" => format!(
            "::iced::window::Position::Specific({})",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?
        ),
        "length.fill" => "::iced::Length::Fill".into(),
        "length.shrink" => "::iced::Length::Shrink".into(),
        "length.fill_portion" => {
            let value = context
                .literal_i64(args.value(0)?)
                .expect("checked u16 literal");
            format!("::iced::Length::FillPortion({value}u16)")
        }
        "length.try_fill_portion" => format!(
            "<u16>::try_from({}).ok().map(::iced::Length::FillPortion)",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?
        ),
        "length.fixed" => format!(
            "::iced::Length::Fixed(({}) as f32)",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?
        ),
        "length.from_f64" => format!(
            "::iced::Length::from(({}) as f32)",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?
        ),
        "length.from_pixels" => format!(
            "::iced::Length::from({})",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?
        ),
        "length.from_u32" => {
            let value = context
                .literal_i64(args.value(0)?)
                .expect("checked u32 literal");
            format!("::iced::Length::from({value}u32)")
        }
        "length.try_from_u32" => format!(
            "<u32>::try_from({}).ok().map(::iced::Length::from)",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?
        ),
        "length.fluid" => format!(
            "({}).fluid()",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?
        ),
        "length.enclose" => format!(
            "({}).enclose({})",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?,
            expr_node_code(args.value(1)?, env, context, ValueMode::Owned)?
        ),
        "alignment.start" => "::iced::Alignment::Start".into(),
        "alignment.center" => "::iced::Alignment::Center".into(),
        "alignment.end" => "::iced::Alignment::End".into(),
        "horizontal.left" => "::iced::alignment::Horizontal::Left".into(),
        "horizontal.center" => "::iced::alignment::Horizontal::Center".into(),
        "horizontal.right" => "::iced::alignment::Horizontal::Right".into(),
        "vertical.top" => "::iced::alignment::Vertical::Top".into(),
        "vertical.center" => "::iced::alignment::Vertical::Center".into(),
        "vertical.bottom" => "::iced::alignment::Vertical::Bottom".into(),
        "alignment.from_horizontal" | "alignment.from_vertical" => format!(
            "::iced::Alignment::from({})",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?
        ),
        "horizontal.from_alignment" => format!(
            "::iced::alignment::Horizontal::from({})",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?
        ),
        "vertical.from_alignment" => format!(
            "::iced::alignment::Vertical::from({})",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?
        ),
        _ => return Ok(None),
    }))
}
fn expr_builtin_group_3(
    name: &str,
    args: &ExprArguments,
    env: &dyn BindingEnvironment,
    context: &ExprEmission<'_>,
    mode: ValueMode,
) -> Result<Option<String>, Error> {
    Ok(Some(match name {
        "border.default" => "::iced::Border::default()".into(),
        "border.new" => format!(
            "::iced::Border {{ color: {}, width: {}, radius: {} }}",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?,
            node_pixel_scalar_code(args.value(1)?, env, context)?,
            node_radius_value_code(args.value(2)?, env, context)?
        ),
        "border.color" => format!(
            "::iced::border::color({})",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?
        ),
        "border.width" => format!(
            "::iced::border::width({})",
            node_pixel_value_code(args.value(0)?, env, context)?
        ),
        "border.rounded" => format!(
            "::iced::border::rounded({})",
            node_radius_value_code(args.value(0)?, env, context)?
        ),
        "border.with_color" => format!(
            "({}).color({})",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?,
            expr_node_code(args.value(1)?, env, context, ValueMode::Owned)?
        ),
        "border.with_width" => format!(
            "({}).width({})",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?,
            node_pixel_value_code(args.value(1)?, env, context)?
        ),
        "border.with_radius" => format!(
            "({}).rounded({})",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?,
            node_radius_value_code(args.value(1)?, env, context)?
        ),
        "radius" => format!(
            "::iced::border::radius({})",
            node_pixel_value_code(args.value(0)?, env, context)?
        ),
        "radius.new" => format!(
            "::iced::border::Radius::new({})",
            node_pixel_value_code(args.value(0)?, env, context)?
        ),
        "radius.default" => "::iced::border::Radius::default()".into(),
        "radius.top_left"
        | "radius.top_right"
        | "radius.bottom_right"
        | "radius.bottom_left"
        | "radius.top"
        | "radius.bottom"
        | "radius.left"
        | "radius.right" => {
            let function = name.strip_prefix("radius.").expect("checked prefix");
            format!(
                "::iced::border::{function}({})",
                node_pixel_value_code(args.value(0)?, env, context)?
            )
        }
        "radius.with_top_left"
        | "radius.with_top_right"
        | "radius.with_bottom_right"
        | "radius.with_bottom_left"
        | "radius.with_top"
        | "radius.with_bottom"
        | "radius.with_left"
        | "radius.with_right" => {
            let method = name.strip_prefix("radius.with_").expect("checked prefix");
            format!(
                "({}).{method}({})",
                expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?,
                node_pixel_value_code(args.value(1)?, env, context)?
            )
        }
        "radius.from_f64" => format!(
            "::iced::border::Radius::from(({}) as f32)",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?
        ),
        "radius.from_u8" | "radius.from_u32" => {
            let value = context
                .literal_i64(args.value(0)?)
                .expect("checked radius integer literal");
            let ty = name.strip_prefix("radius.from_").expect("checked prefix");
            format!("::iced::border::Radius::from({value}{ty})")
        }
        "radius.from_i32" => format!(
            "::iced::border::Radius::from(({}) as i32)",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?
        ),
        "radius.try_from_u8" | "radius.try_from_u32" | "radius.try_from_i32" => {
            let ty = name
                .strip_prefix("radius.try_from_")
                .expect("checked prefix");
            format!(
                "<{ty}>::try_from(({}) as i64).ok().map(::iced::border::Radius::from)",
                expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?
            )
        }
        "shadow.default" => "::iced::Shadow::default()".into(),
        "shadow.new" => format!(
            "::iced::Shadow {{ color: {}, offset: {}, blur_radius: ({}) as f32 }}",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?,
            expr_node_code(args.value(1)?, env, context, ValueMode::Owned)?,
            expr_node_code(args.value(2)?, env, context, ValueMode::Owned)?
        ),
        "fit.default" => "::iced::ContentFit::default()".into(),
        "fit.contain" => "::iced::ContentFit::Contain".into(),
        "fit.cover" => "::iced::ContentFit::Cover".into(),
        "fit.fill" => "::iced::ContentFit::Fill".into(),
        "fit.none" => "::iced::ContentFit::None".into(),
        "fit.scale_down" => "::iced::ContentFit::ScaleDown".into(),
        "fit.apply" => format!(
            "({}).fit({}, {})",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?,
            expr_node_code(args.value(1)?, env, context, ValueMode::Owned)?,
            expr_node_code(args.value(2)?, env, context, ValueMode::Owned)?
        ),
        "rotation.default" => "::iced::Rotation::default()".into(),
        "rotation.floating" => format!(
            "::iced::Rotation::Floating({})",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?
        ),
        "rotation.solid" => format!(
            "::iced::Rotation::Solid({})",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?
        ),
        "rotation.from" => format!(
            "::iced::Rotation::from({} as f32)",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?
        ),
        "rotation.with_radians" => format!(
            "{{ let mut __rotation = {}; *__rotation.radians_mut() = {}; __rotation }}",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?,
            expr_node_code(args.value(1)?, env, context, ValueMode::Owned)?
        ),
        "rotation.apply" => format!(
            "({}).apply({})",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?,
            expr_node_code(args.value(1)?, env, context, ValueMode::Owned)?
        ),
        "debug.active" => format!(
            "({}).is_some()",
            expr_node_code(args.value(0)?, env, context, ValueMode::Borrowed)?
        ),
        "debug.time_with" => format!(
            "::iced::debug::time_with({}, || {})",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?,
            expr_node_code(args.value(1)?, env, context, mode)?
        ),
        "image.downgrade" => format!(
            "({}).downgrade()",
            expr_node_code(args.value(0)?, env, context, ValueMode::Borrowed)?
        ),
        "image.upgrade" => format!(
            "::iced::widget::image::Allocation::upgrade(&({}))",
            expr_node_code(args.value(0)?, env, context, ValueMode::Borrowed)?
        ),
        "animation.value" => {
            let animation = expr_node_code(args.value(0)?, env, context, ValueMode::Borrowed)?;
            if context.ty(args.value(0)?, env)? == Type::Animation(Box::new(Type::F64)) {
                format!("({animation}).value() as f64")
            } else {
                format!("({animation}).value()")
            }
        }
        "animation.animating" => format!(
            "({}).is_animating({})",
            expr_node_code(args.value(0)?, env, context, ValueMode::Borrowed)?,
            expr_animation_at_code(args, 1, env, context)?
        ),
        "animation.interpolate" => {
            let animation = expr_node_code(args.value(0)?, env, context, ValueMode::Borrowed)?;
            let start = expr_node_code(args.value(1)?, env, context, ValueMode::Owned)?;
            let end = expr_node_code(args.value(2)?, env, context, ValueMode::Owned)?;
            let at = expr_animation_at_code(args, 3, env, context)?;
            if context.ty(args.value(1)?, env)? == Type::F64 {
                format!("({animation}).interpolate(({start}) as f32, ({end}) as f32, {at}) as f64")
            } else {
                let start = if context.is_none(args.value(1)?) {
                    "::std::option::Option::<f32>::None".into()
                } else {
                    format!("({start}).map(|__value| __value as f32)")
                };
                let end = if context.is_none(args.value(2)?) {
                    "::std::option::Option::<f32>::None".into()
                } else {
                    format!("({end}).map(|__value| __value as f32)")
                };
                format!(
                    "({animation}).interpolate({start}, {end}, {at}).map(|__value| __value as f64)"
                )
            }
        }
        "animation.remaining" => format!(
            "::ui_lang_runtime::animation_remaining_millis(&({}), {})",
            expr_node_code(args.value(0)?, env, context, ValueMode::Borrowed)?,
            expr_animation_at_code(args, 1, env, context)?
        ),
        "animation.project" => {
            let animation = expr_node_code(args.value(0)?, env, context, ValueMode::Borrowed)?;
            let Type::Animation(inner) = context.ty(args.value(0)?, env)? else {
                unreachable!("checker requires animation")
            };
            let (binding, binding_id) = args.binding(1, context)?;
            let projection_env = LayeredBindingEnv::new(
                env,
                binding,
                Binding {
                    code: if *inner == Type::F64 {
                        "(__value as f64)".into()
                    } else {
                        "__value".into()
                    },
                    ty: *inner,
                    local: true,
                    state: None,
                    owner: binding_id.map(BindingOwner::Local),
                },
            );
            let projection =
                expr_node_code(args.value(2)?, &projection_env, context, ValueMode::Owned)?;
            let output = context.ty(args.value(2)?, &projection_env)?;
            let at = expr_animation_at_code(args, 3, env, context)?;
            if output == Type::F64 {
                format!(
                    "({animation}).interpolate_with(|__value| ({projection}) as f32, {at}) as f64"
                )
            } else {
                format!(
                    "({animation}).interpolate_with(|__value| ({projection}).map(|__value| __value as f32), {at}).map(|__value| __value as f64)"
                )
            }
        }
        "pixels" => format!(
            "::iced::Pixels(({}) as f32)",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?
        ),
        "pixels.zero" => "::iced::Pixels::ZERO".into(),
        "pixels.from_u32" => {
            let value = context
                .literal_i64(args.value(0)?)
                .expect("checked pixels u32 literal");
            format!("::iced::Pixels::from({value}u32)")
        }
        "pixels.try_from_u32" => format!(
            "<u32>::try_from({}).ok().map(::iced::Pixels::from)",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?
        ),
        "padding" => format!(
            "::iced::Padding {{ top: ({}) as f32, right: ({}) as f32, bottom: ({}) as f32, left: ({}) as f32 }}",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?,
            expr_node_code(args.value(1)?, env, context, ValueMode::Owned)?,
            expr_node_code(args.value(2)?, env, context, ValueMode::Owned)?,
            expr_node_code(args.value(3)?, env, context, ValueMode::Owned)?
        ),
        "padding.zero" => "::iced::Padding::ZERO".into(),
        "padding.all" | "padding.top" | "padding.right" | "padding.bottom" | "padding.left"
        | "padding.horizontal" | "padding.vertical" => {
            let function = name.strip_prefix("padding.").expect("checked prefix");
            format!(
                "::iced::padding::{function}({})",
                node_pixel_value_code(args.value(0)?, env, context)?
            )
        }
        "padding.axes" => format!(
            "::iced::Padding::from([({}) as f32, ({}) as f32])",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?,
            expr_node_code(args.value(1)?, env, context, ValueMode::Owned)?
        ),
        "padding.from_pixels" => format!(
            "::iced::Padding::from({})",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?
        ),
        "padding.with_top"
        | "padding.with_right"
        | "padding.with_bottom"
        | "padding.with_left"
        | "padding.with_horizontal"
        | "padding.with_vertical" => {
            let method = name.strip_prefix("padding.with_").expect("checked prefix");
            format!(
                "({}).{method}({})",
                expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?,
                node_pixel_value_code(args.value(1)?, env, context)?
            )
        }
        "padding.fit" => format!(
            "({}).fit({}, {})",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?,
            expr_node_code(args.value(1)?, env, context, ValueMode::Owned)?,
            expr_node_code(args.value(2)?, env, context, ValueMode::Owned)?
        ),
        "degrees" => format!(
            "::iced::Degrees(({}) as f32)",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?
        ),
        "radians" => format!(
            "::iced::Radians(({}) as f32)",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?
        ),
        "degrees.range_start" => "*::iced::Degrees::RANGE.start()".into(),
        "degrees.range_end" => "*::iced::Degrees::RANGE.end()".into(),
        "radians.range_start" => "*::iced::Radians::RANGE.start()".into(),
        "radians.range_end" => "*::iced::Radians::RANGE.end()".into(),
        "radians.pi" => "::iced::Radians::PI".into(),
        "degrees.in_range" => format!(
            "::iced::Degrees::RANGE.contains(&({}))",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?
        ),
        "radians.in_range" => format!(
            "::iced::Radians::RANGE.contains(&({}))",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?
        ),
        "radians.from_degrees" => format!(
            "::iced::Radians::from({})",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?
        ),
        "radians.distance_start" | "radians.distance_end" => format!(
            "({}).to_distance(&({})).{}",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?,
            expr_node_code(args.value(1)?, env, context, ValueMode::Owned)?,
            if name == "radians.distance_start" {
                0
            } else {
                1
            }
        ),
        "point" => format!(
            "::iced::Point::new(({}) as f32, ({}) as f32)",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?,
            expr_node_code(args.value(1)?, env, context, ValueMode::Owned)?
        ),
        "vector" => format!(
            "::iced::Vector::new(({}) as f32, ({}) as f32)",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?,
            expr_node_code(args.value(1)?, env, context, ValueMode::Owned)?
        ),
        "size" => format!(
            "::iced::Size::new(({}) as f32, ({}) as f32)",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?,
            expr_node_code(args.value(1)?, env, context, ValueMode::Owned)?
        ),
        "rectangle" => format!(
            "::iced::Rectangle {{ x: ({}) as f32, y: ({}) as f32, width: ({}) as f32, height: ({}) as f32 }}",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?,
            expr_node_code(args.value(1)?, env, context, ValueMode::Owned)?,
            expr_node_code(args.value(2)?, env, context, ValueMode::Owned)?,
            expr_node_code(args.value(3)?, env, context, ValueMode::Owned)?
        ),
        _ => return Ok(None),
    }))
}
fn expr_builtin_group_4(
    name: &str,
    args: &ExprArguments,
    env: &dyn BindingEnvironment,
    context: &ExprEmission<'_>,
    _mode: ValueMode,
) -> Result<Option<String>, Error> {
    Ok(Some(match name {
        "point.origin" => "::iced::Point::ORIGIN".into(),
        "point.distance" => format!(
            "({}).distance({}) as f64",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?,
            expr_node_code(args.value(1)?, env, context, ValueMode::Owned)?
        ),
        "point.snap" => format!(
            "({}).snap()",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?
        ),
        "vector.zero" => "::iced::Vector::ZERO".into(),
        "size.zero" => "::iced::Size::ZERO".into(),
        "size.unit" => "::iced::Size::UNIT".into(),
        "size.infinite" => "::iced::Size::INFINITE".into(),
        "size.min" => format!(
            "({}).min({})",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?,
            expr_node_code(args.value(1)?, env, context, ValueMode::Owned)?
        ),
        "size.max" => format!(
            "({}).max({})",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?,
            expr_node_code(args.value(1)?, env, context, ValueMode::Owned)?
        ),
        "size.expand" => format!(
            "({}).expand({})",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?,
            expr_node_code(args.value(1)?, env, context, ValueMode::Owned)?
        ),
        "size.rotate" => format!(
            "({}).rotate({})",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?,
            node_radians_value_code(args.value(1)?, env, context)?
        ),
        "size.ratio" => format!(
            "({}).ratio(({}) as f32)",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?,
            expr_node_code(args.value(1)?, env, context, ValueMode::Owned)?
        ),
        "size.from_vector" => format!(
            "::iced::Size::from({})",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?
        ),
        "vector.from_size" => format!(
            "::iced::Vector::from({})",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?
        ),
        "size.from_padding" => format!(
            "::iced::Size::from({})",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?
        ),
        "size.from_u32" => {
            let width = context
                .literal_i64(args.value(0)?)
                .expect("checked size width literal");
            let height = context
                .literal_i64(args.value(1)?)
                .expect("checked size height literal");
            format!("::iced::Size::from(({width}u32, {height}u32))")
        }
        "size.try_from_u32" => format!(
            "match (<u32>::try_from({}), <u32>::try_from({})) {{ (::std::result::Result::Ok(width), ::std::result::Result::Ok(height)) => ::std::option::Option::Some(::iced::Size::from((width, height))), _ => ::std::option::Option::None }}",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?,
            expr_node_code(args.value(1)?, env, context, ValueMode::Owned)?
        ),
        "rectangle.zero" => "::iced::Rectangle::default()".into(),
        "rectangle.infinite" => "::iced::Rectangle::INFINITE".into(),
        "rectangle.with_size" => format!(
            "::iced::Rectangle::with_size({})",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?
        ),
        "rectangle.with_radius" => format!(
            "::iced::Rectangle::with_radius(({}) as f32)",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?
        ),
        "rectangle.with_vertices" => format!(
            "::iced::Rectangle::with_vertices({}, {}, {}).0",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?,
            expr_node_code(args.value(1)?, env, context, ValueMode::Owned)?,
            expr_node_code(args.value(2)?, env, context, ValueMode::Owned)?
        ),
        "rectangle.vertices_rotation" => format!(
            "::iced::Rectangle::with_vertices({}, {}, {}).1.0 as f64",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?,
            expr_node_code(args.value(1)?, env, context, ValueMode::Owned)?,
            expr_node_code(args.value(2)?, env, context, ValueMode::Owned)?
        ),
        "rectangle.vertices_angle" => format!(
            "::iced::Rectangle::with_vertices({}, {}, {}).1",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?,
            expr_node_code(args.value(1)?, env, context, ValueMode::Owned)?,
            expr_node_code(args.value(2)?, env, context, ValueMode::Owned)?
        ),
        "rectangle.contains" => format!(
            "({}).contains({})",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?,
            expr_node_code(args.value(1)?, env, context, ValueMode::Owned)?
        ),
        "rectangle.distance" => format!(
            "({}).distance({}) as f64",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?,
            expr_node_code(args.value(1)?, env, context, ValueMode::Owned)?
        ),
        "rectangle.offset" => format!(
            "({}).offset(&({}))",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?,
            expr_node_code(args.value(1)?, env, context, ValueMode::Owned)?
        ),
        "rectangle.is_within" => format!(
            "({}).is_within(&({}))",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?,
            expr_node_code(args.value(1)?, env, context, ValueMode::Owned)?
        ),
        "rectangle.intersection" => format!(
            "({}).intersection(&({}))",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?,
            expr_node_code(args.value(1)?, env, context, ValueMode::Owned)?
        ),
        "rectangle.intersects" => format!(
            "({}).intersects(&({}))",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?,
            expr_node_code(args.value(1)?, env, context, ValueMode::Owned)?
        ),
        "rectangle.union" => format!(
            "({}).union(&({}))",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?,
            expr_node_code(args.value(1)?, env, context, ValueMode::Owned)?
        ),
        "rectangle.snap" => format!(
            "({}).snap()",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?
        ),
        "rectangle.expand" | "rectangle.shrink" => format!(
            "({}).{}(::iced::Padding {{ top: ({}) as f32, right: ({}) as f32, bottom: ({}) as f32, left: ({}) as f32 }})",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?,
            if name == "rectangle.expand" {
                "expand"
            } else {
                "shrink"
            },
            expr_node_code(args.value(1)?, env, context, ValueMode::Owned)?,
            expr_node_code(args.value(2)?, env, context, ValueMode::Owned)?,
            expr_node_code(args.value(3)?, env, context, ValueMode::Owned)?,
            expr_node_code(args.value(4)?, env, context, ValueMode::Owned)?
        ),
        "rectangle.expand_padding" | "rectangle.shrink_padding" => format!(
            "({}).{}({})",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?,
            if name == "rectangle.expand_padding" {
                "expand"
            } else {
                "shrink"
            },
            expr_node_code(args.value(1)?, env, context, ValueMode::Owned)?
        ),
        "rectangle.rotate" => format!(
            "({}).rotate({})",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?,
            node_radians_value_code(args.value(1)?, env, context)?
        ),
        "rectangle.zoom" => format!(
            "({}).zoom(({}) as f32)",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?,
            expr_node_code(args.value(1)?, env, context, ValueMode::Owned)?
        ),
        "rectangle.anchor" => {
            let horizontal = context
                .literal_str(args.value(2)?)
                .expect("checked horizontal alignment literal");
            let vertical = context
                .literal_str(args.value(3)?)
                .expect("checked vertical alignment literal");
            let horizontal = match horizontal {
                "left" => "Left",
                "center" => "Center",
                "right" => "Right",
                _ => unreachable!("checker validates horizontal alignment"),
            };
            let vertical = match vertical {
                "top" => "Top",
                "center" => "Center",
                "bottom" => "Bottom",
                _ => unreachable!("checker validates vertical alignment"),
            };
            format!(
                "({}).anchor({}, ::iced::alignment::Horizontal::{horizontal}, ::iced::alignment::Vertical::{vertical})",
                expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?,
                expr_node_code(args.value(1)?, env, context, ValueMode::Owned)?
            )
        }
        "rectangle.from_u32" => format!(
            "::iced::Rectangle::from({})",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?
        ),
        "transform.identity" => "::iced::Transformation::IDENTITY".into(),
        "transform.orthographic" => {
            let width = context
                .literal_i64(args.value(0)?)
                .expect("checked orthographic width literal");
            let height = context
                .literal_i64(args.value(1)?)
                .expect("checked orthographic height literal");
            format!("::iced::Transformation::orthographic({width}u32, {height}u32)")
        }
        "transform.try_orthographic" => format!(
            "match (<u32>::try_from({}), <u32>::try_from({})) {{ (::std::result::Result::Ok(width), ::std::result::Result::Ok(height)) => ::std::option::Option::Some(::iced::Transformation::orthographic(width, height)), _ => ::std::option::Option::None }}",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?,
            expr_node_code(args.value(1)?, env, context, ValueMode::Owned)?
        ),
        "transform.translate" => format!(
            "::iced::Transformation::translate(({}) as f32, ({}) as f32)",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?,
            expr_node_code(args.value(1)?, env, context, ValueMode::Owned)?
        ),
        "transform.scale" => format!(
            "::iced::Transformation::scale(({}) as f32)",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?
        ),
        "transform.inverse" => format!(
            "({}).inverse()",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?
        ),
        "transform.compose" => format!(
            "({}) * ({})",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?,
            expr_node_code(args.value(1)?, env, context, ValueMode::Owned)?
        ),
        "transform.point"
        | "transform.vector"
        | "transform.size"
        | "transform.rectangle"
        | "transform.cursor"
        | "transform.click" => format!(
            "({}) * ({})",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?,
            expr_node_code(args.value(1)?, env, context, ValueMode::Owned)?
        ),
        _ => return Ok(None),
    }))
}
fn expr_builtin_group_5(
    name: &str,
    args: &ExprArguments,
    env: &dyn BindingEnvironment,
    context: &ExprEmission<'_>,
    _mode: ValueMode,
) -> Result<Option<String>, Error> {
    Ok(Some(match name {
            "mouse.button" => {
                let value = context
                    .literal_str(args.value(0)?)
                    .expect("checked mouse button literal");
                let variant = match value {
                    "left" => "Left",
                    "right" => "Right",
                    "middle" => "Middle",
                    "back" => "Back",
                    "forward" => "Forward",
                    _ => unreachable!("checker validates mouse buttons"),
                };
                format!("::iced::mouse::Button::{variant}")
            }
            "mouse.other_button" => {
                let value = context
                    .literal_i64(args.value(0)?)
                    .expect("checked mouse button literal");
                format!("::iced::mouse::Button::Other({value}u16)")
            }
            "mouse.try_other_button" => format!(
                "<u16>::try_from({}).ok().map(::iced::mouse::Button::Other)",
                expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?
            ),
            "mouse.cursor" => format!(
                "::iced::mouse::Cursor::Available({})",
                expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?
            ),
            "mouse.levitating" => format!(
                "::iced::mouse::Cursor::Levitating({})",
                expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?
            ),
            "mouse.unavailable" => "::iced::mouse::Cursor::Unavailable".into(),
            "mouse.cursor_position" => format!(
                "({}).position()",
                expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?
            ),
            "mouse.cursor_over" => format!(
                "({}).position_over({})",
                expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?,
                expr_node_code(args.value(1)?, env, context, ValueMode::Owned)?
            ),
            "mouse.cursor_in" => format!(
                "({}).position_in({})",
                expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?,
                expr_node_code(args.value(1)?, env, context, ValueMode::Owned)?
            ),
            "mouse.cursor_from" => format!(
                "({}).position_from({})",
                expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?,
                expr_node_code(args.value(1)?, env, context, ValueMode::Owned)?
            ),
            "mouse.cursor_is_over" => format!(
                "({}).is_over({})",
                expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?,
                expr_node_code(args.value(1)?, env, context, ValueMode::Owned)?
            ),
            "mouse.cursor_is_levitating" => format!(
                "({}).is_levitating()",
                expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?
            ),
            "mouse.cursor_levitate" => format!(
                "({}).levitate()",
                expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?
            ),
            "mouse.cursor_land" => format!(
                "({}).land()",
                expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?
            ),
            "mouse.cursor_translate" => format!(
                "({}) + ::iced::Vector::new(({}) as f32, ({}) as f32)",
                expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?,
                expr_node_code(args.value(1)?, env, context, ValueMode::Owned)?,
                expr_node_code(args.value(2)?, env, context, ValueMode::Owned)?
            ),
            "mouse.click" => format!(
                "::iced::advanced::mouse::Click::new({}, {}, {})",
                expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?,
                expr_node_code(args.value(1)?, env, context, ValueMode::Owned)?,
                expr_node_code(args.value(2)?, env, context, ValueMode::Owned)?
            ),
            "touch.finger" => {
                let value = context
                    .literal_str(args.value(0)?)
                    .expect("checked touch finger literal");
                let value = value
                    .parse::<u64>()
                    .expect("checker validates touch finger literals");
                format!("::iced::touch::Finger({value}u64)")
            }
            "touch.try_finger" => format!(
                "({}).parse::<u64>().ok().map(::iced::touch::Finger)",
                expr_node_code(args.value(0)?, env, context, ValueMode::Borrowed)?
            ),
            "key.named" => {
                let variant = context
                    .literal_str(args.value(0)?)
                    .expect("checked named key variant");
                format!(
                    "::iced::keyboard::Key::Named(::iced::keyboard::key::Named::{variant})"
                )
            }
            "key.character" => format!(
                "::iced::keyboard::Key::Character(({}).into())",
                expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?
            ),
            "key.unidentified" => "::iced::keyboard::Key::Unidentified".into(),
            "key.code" => {
                let variant = context
                    .literal_str(args.value(0)?)
                    .expect("checked physical key variant");
                format!(
                    "::iced::keyboard::key::Physical::Code(::iced::keyboard::key::Code::{variant})"
                )
            }
            "key.native_unidentified" => "::iced::keyboard::key::Physical::Unidentified(::iced::keyboard::key::NativeCode::Unidentified)".into(),
            "key.command_modifiers" => "::iced::keyboard::Modifiers::COMMAND".into(),
            "key.native" => {
                let platform = context
                    .literal_str(args.value(0)?)
                    .expect("checked native key platform");
                let value = context
                    .literal_i64(args.value(1)?)
                    .expect("checked native key literal");
                let (variant, ty) = match platform {
                    "android" => ("Android", "u32"),
                    "macos" => ("MacOS", "u16"),
                    "windows" => ("Windows", "u16"),
                    "xkb" => ("Xkb", "u32"),
                    _ => unreachable!("checker validates native key platforms"),
                };
                format!(
                    "::iced::keyboard::key::Physical::Unidentified(::iced::keyboard::key::NativeCode::{variant}({value}{ty}))"
                )
            }
            "key.try_native" => {
                let platform = context
                    .literal_str(args.value(0)?)
                    .expect("checked native key platform");
                let (variant, ty) = match platform {
                    "android" => ("Android", "u32"),
                    "macos" => ("MacOS", "u16"),
                    "windows" => ("Windows", "u16"),
                    "xkb" => ("Xkb", "u32"),
                    _ => unreachable!("checker validates native key platforms"),
                };
                format!(
                    "<{ty}>::try_from({}).ok().map(|value| ::iced::keyboard::key::Physical::Unidentified(::iced::keyboard::key::NativeCode::{variant}(value)))",
                    expr_node_code(args.value(1)?, env, context, ValueMode::Owned)?
                )
            }
            "key.location" => {
                let value = context
                    .literal_str(args.value(0)?)
                    .expect("checked key location literal");
                let variant = match value {
                    "standard" => "Standard",
                    "left" => "Left",
                    "right" => "Right",
                    "numpad" => "Numpad",
                    _ => unreachable!("checker validates key locations"),
                };
                format!("::iced::keyboard::Location::{variant}")
            }
            "key.modifiers" => {
                let values = ["SHIFT", "CTRL", "ALT", "LOGO"]
                    .into_iter()
                    .zip(args.values()?)
                    .map(|(flag, value)| {
                        Ok(format!(
                            "if {} {{ ::iced::keyboard::Modifiers::{flag} }} else {{ ::iced::keyboard::Modifiers::empty() }}",
                            expr_node_code(value, env, context, ValueMode::Owned)?
                        ))
                    })
                    .collect::<Result<Vec<_>, Error>>()?;
                format!("({})", values.join(" | "))
            }
            "key.latin" => format!(
                "({}).to_latin({}).map(|value| value.to_string())",
                expr_node_code(args.value(0)?, env, context, ValueMode::Borrowed)?,
                expr_node_code(args.value(1)?, env, context, ValueMode::Owned)?
            ),
            "len" => format!(
                "({}).len() as i64",
                expr_node_code(args.value(0)?, env, context, ValueMode::Borrowed)?
            ),
            "empty" => format!(
                "({}).is_empty()",
                expr_node_code(args.value(0)?, env, context, ValueMode::Borrowed)?
            ),
            _ => return Ok(None),
    }))
}
fn expr_builtin_group_6(
    name: &str,
    args: &ExprArguments,
    env: &dyn BindingEnvironment,
    context: &ExprEmission<'_>,
    _mode: ValueMode,
) -> Result<Option<String>, Error> {
    let document = context.document;
    Ok(Some(match name {
        "trim" => format!(
            "({}).trim().to_owned()",
            expr_node_code(args.value(0)?, env, context, ValueMode::Borrowed)?
        ),
        "some" => format!(
            "::std::option::Option::Some({})",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?
        ),
        "ok" => format!(
            "::std::result::Result::Ok({})",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?
        ),
        "err" => format!(
            "::std::result::Result::Err({})",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?
        ),
        "markdown" => format!(
            "::iced::widget::markdown::Content::parse(&{})",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?
        ),
        "markdown_images" => format!(
            "({}).images().iter().cloned().collect::<::std::vec::Vec<_>>()",
            expr_node_code(args.value(0)?, env, context, ValueMode::Borrowed)?
        ),
        "editor" => format!(
            "::iced::widget::text_editor::Content::with_text(&{})",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?
        ),
        "editor_text" => format!(
            "({}).text()",
            expr_node_code(args.value(0)?, env, context, ValueMode::Borrowed)?
        ),
        "editor_copy" => {
            let source = expr_node_code(args.value(0)?, env, context, ValueMode::Borrowed)?;
            format!(
                "{{ let __source = &{source}; let mut __copy = ::iced::widget::text_editor::Content::with_text(&__source.text()); __copy.move_to(__source.cursor()); __copy }}"
            )
        }
        "editor_cursor_line" => format!(
            "(({}).cursor().position.line.min(i64::MAX as usize) as i64)",
            expr_node_code(args.value(0)?, env, context, ValueMode::Borrowed)?
        ),
        "editor_cursor_column" => format!(
            "(({}).cursor().position.column.min(i64::MAX as usize) as i64)",
            expr_node_code(args.value(0)?, env, context, ValueMode::Borrowed)?
        ),
        "editor_line_count" => format!(
            "(({}).line_count().min(i64::MAX as usize) as i64)",
            expr_node_code(args.value(0)?, env, context, ValueMode::Borrowed)?
        ),
        "editor_has_selection" => format!(
            "({}).cursor().selection.is_some()",
            expr_node_code(args.value(0)?, env, context, ValueMode::Borrowed)?
        ),
        "editor_line" => format!(
            "::std::convert::TryFrom::try_from({}).ok().and_then(|__line| ({}).line(__line)).map(|__line| __line.text.into_owned())",
            expr_node_code(args.value(1)?, env, context, ValueMode::Owned)?,
            expr_node_code(args.value(0)?, env, context, ValueMode::Borrowed)?
        ),
        "encoded" => format!(
            "::iced::widget::image::Handle::from_bytes({})",
            expr_node_code(args.value(0)?, env, context, ValueMode::Owned)?
        ),
        "rgba" => format!(
            "::iced::widget::image::Handle::from_rgba({}, {}, {})",
            node_u32_code(args.value(0)?, env, context)?,
            node_u32_code(args.value(1)?, env, context)?,
            expr_node_code(args.value(2)?, env, context, ValueMode::Owned)?
        ),
        "aborted" => format!(
            "({}).as_ref().is_some_and(::iced::task::Handle::is_aborted)",
            expr_node_code(args.value(0)?, env, context, ValueMode::Borrowed)?
        ),
        _ => {
            let function = find_extern_function(document, name, ExternKind::Sync)
                .expect("checker accepts only declared sync calls");
            let args = expr_node_list_code(&args.values()?, env, context)?;
            format!("{}({args})", function.rust_path)
        }
    }))
}

fn expr_node_list_code(
    values: &[ExprNode],
    env: &dyn BindingEnvironment,
    context: &ExprEmission<'_>,
) -> Result<String, Error> {
    Ok(values
        .iter()
        .map(|value| expr_node_code(*value, env, context, ValueMode::Owned))
        .collect::<Result<Vec<_>, _>>()?
        .join(", "))
}

fn checked_path_code(
    root: &CheckedPathRoot,
    projections: &[CheckedProjection],
    env: &dyn BindingEnvironment,
    context: &ExprEmission<'_>,
    mode: ValueMode,
) -> Result<String, Error> {
    let program = context.program.expect("checked path has a lowered program");
    let (mut code, mut ty, root_local) = match root {
        CheckedPathRoot::Value(value_ref) => {
            let value = program.checked_facts().value_by_ref(*value_ref);
            let origin = program.origin(value.origin);
            let span = Span {
                line: origin.line,
                column: origin.column,
            };
            let binding = env.get(&value.name).ok_or_else(|| {
                Error::new(
                    "E196",
                    &span,
                    format!(
                        "normalized value `{}` is absent from emission scope",
                        value.name
                    ),
                )
            })?;
            if binding.owner != Some(BindingOwner::Value(*value_ref)) {
                return Err(Error::new(
                    "E196",
                    &span,
                    format!(
                        "normalized value `{}` resolved to a mismatched emission owner",
                        value.name
                    ),
                ));
            }
            (binding.code.clone(), binding.ty.clone(), binding.local)
        }
        CheckedPathRoot::Local(id) => {
            let local = program.checked_facts().local(*id);
            let origin = program.origin(local.origin);
            let span = Span {
                line: origin.line,
                column: origin.column,
            };
            let binding = env.get(&local.name).ok_or_else(|| {
                Error::new(
                    "E196",
                    &span,
                    format!(
                        "normalized local `{}` is absent from emission scope",
                        local.name
                    ),
                )
            })?;
            if binding.owner != Some(BindingOwner::Local(*id)) {
                return Err(Error::new(
                    "E196",
                    &span,
                    format!(
                        "normalized local `{}` resolved to a mismatched emission owner",
                        local.name
                    ),
                ));
            }
            (binding.code.clone(), binding.ty.clone(), binding.local)
        }
        CheckedPathRoot::EnumVariant(id) => {
            let variant = program
                .declarations()
                .try_enum_variant_decl(*id)
                .ok_or_else(|| {
                    Error::new(
                        "E196",
                        &Span::line(1),
                        "checked path references an invalid enum variant ID",
                    )
                })?;
            let owner = program
                .declarations()
                .try_enum_decl(id.owner)
                .ok_or_else(|| {
                    Error::new(
                        "E196",
                        &Span::line(1),
                        "checked path references an invalid enum ID",
                    )
                })?;
            return Ok(format!("{}::{}", owner.rust_name, pascal(&variant.name)));
        }
        CheckedPathRoot::Palette(id) => {
            let palette = &program.theme().palettes[id.0 as usize];
            return Ok(format!(
                "{}::{}",
                generated_named_rust(&program.theme().contract.name),
                pascal(&palette.name)
            ));
        }
    };

    let mut owned_projection = false;
    for projection in projections {
        match projection.kind {
            crate::check::CheckedProjectionKind::Struct(_) => {
                write!(code, ".{}", projection.field).unwrap();
            }
            crate::check::CheckedProjectionKind::OptionalWidgetTarget => {
                code = format!(
                    "({code}).as_ref().map(|value| value.{}.clone())",
                    projection.field
                );
                owned_projection = true;
            }
            crate::check::CheckedProjectionKind::Native => {
                if let Some((native, _)) =
                    native_field_projection(&projection.input, &projection.field, &code)
                {
                    code = native;
                    owned_projection = true;
                } else {
                    write!(code, ".{}", projection.field).unwrap();
                }
            }
        }
        ty = projection.output.clone();
    }
    if matches!(mode, ValueMode::Owned) && !copy_expression_type(&ty) {
        let already_owned = owned_projection || (root_local && projections.is_empty());
        if !already_owned {
            if ty == Type::Str {
                code.push_str(".to_owned()");
            } else {
                code.push_str(".clone()");
            }
        }
    }
    Ok(code)
}

fn copy_expression_type(ty: &Type) -> bool {
    matches!(
        ty,
        Type::Bool
            | Type::I64
            | Type::F64
            | Type::PhysicalKey
            | Type::KeyLocation
            | Type::KeyModifiers
            | Type::Pixels
            | Type::Padding
            | Type::Degrees
            | Type::Radians
            | Type::Rotation
            | Type::ContentFit
            | Type::Color
            | Type::Background
            | Type::Gradient
            | Type::LinearGradient
            | Type::ColorStop
            | Type::Font
            | Type::FontFamily
            | Type::FontWeight
            | Type::FontStretch
            | Type::FontStyle
            | Type::ThemeMode
            | Type::TextAlignment
            | Type::TextShaping
            | Type::TextWrapping
            | Type::TextLineHeight
            | Type::MouseInteraction
            | Type::ScrollDelta
            | Type::EventStatus
            | Type::Length
            | Type::Alignment
            | Type::HorizontalAlignment
            | Type::VerticalAlignment
            | Type::Border
            | Type::Radius
            | Type::Shadow
            | Type::Point
            | Type::PointU32
            | Type::Vector
            | Type::Size
            | Type::SizeU32
            | Type::Rectangle
            | Type::RectangleU32
            | Type::Transformation
            | Type::MouseButton
            | Type::MouseCursor
            | Type::MouseClick
            | Type::TouchFinger
            | Type::WindowId
            | Type::WindowPosition
            | Type::RedrawRequest
            | Type::WindowDirection
            | Type::WindowLevel
            | Type::WindowMode
            | Type::WindowAttention
            | Type::Unit
    )
}

fn node_unit_f32_code(
    expr: ExprNode,
    env: &dyn BindingEnvironment,
    context: &ExprEmission<'_>,
) -> Result<String, Error> {
    let code = expr_node_code(expr, env, context, ValueMode::Owned)?;
    Ok(format!("(({code}) as f32).max(0.0).min(1.0)"))
}

fn node_pixel_value_code(
    expr: ExprNode,
    env: &dyn BindingEnvironment,
    context: &ExprEmission<'_>,
) -> Result<String, Error> {
    let code = expr_node_code(expr, env, context, ValueMode::Owned)?;
    Ok(if context.ty(expr, env)? == Type::Pixels {
        code
    } else {
        format!("({code}) as f32")
    })
}

fn node_pixel_scalar_code(
    expr: ExprNode,
    env: &dyn BindingEnvironment,
    context: &ExprEmission<'_>,
) -> Result<String, Error> {
    let code = expr_node_code(expr, env, context, ValueMode::Owned)?;
    Ok(if context.ty(expr, env)? == Type::Pixels {
        format!("({code}).0")
    } else {
        format!("({code}) as f32")
    })
}

fn node_radius_value_code(
    expr: ExprNode,
    env: &dyn BindingEnvironment,
    context: &ExprEmission<'_>,
) -> Result<String, Error> {
    let code = expr_node_code(expr, env, context, ValueMode::Owned)?;
    Ok(if context.ty(expr, env)? == Type::Radius {
        code
    } else {
        format!("::iced::border::Radius::from(({code}) as f32)")
    })
}

fn node_radians_value_code(
    expr: ExprNode,
    env: &dyn BindingEnvironment,
    context: &ExprEmission<'_>,
) -> Result<String, Error> {
    let code = expr_node_code(expr, env, context, ValueMode::Owned)?;
    Ok(if context.ty(expr, env)? == Type::Radians {
        code
    } else {
        format!("::iced::Radians(({code}) as f32)")
    })
}

fn node_u32_code(
    expr: ExprNode,
    env: &dyn BindingEnvironment,
    context: &ExprEmission<'_>,
) -> Result<String, Error> {
    Ok(format!(
        "({}).clamp(0, u32::MAX as i64) as u32",
        expr_node_code(expr, env, context, ValueMode::Owned)?
    ))
}

fn expr_animation_at_code(
    args: &ExprArguments,
    index: usize,
    env: &dyn BindingEnvironment,
    context: &ExprEmission<'_>,
) -> Result<String, Error> {
    match args.get(index) {
        None => Ok("::iced::time::Instant::now()".into()),
        Some(ExprArgument::Value(at)) => expr_node_code(at, env, context, ValueMode::Owned),
        Some(ExprArgument::Binding(_)) => Err(Error::new(
            "E196",
            &Span::line(1),
            "animation instant resolved to a binding",
        )),
    }
}

pub(in crate::codegen) fn checked_expr_use_code(
    program: &LoweredProgram,
    expression_use: CheckedExprUseId,
    env: &dyn BindingEnvironment,
    mode: ValueMode,
) -> Result<String, Error> {
    let facts = program.checked_facts();
    let expression_use = facts.expression_use(expression_use);
    let context = ExprEmission::for_checked(program);
    let code = expr_node_code(ExprNode::Checked(expression_use.root), env, &context, mode)?;
    Ok(match &expression_use.coercion {
        CheckedInitializerCoercion::None => code,
        CheckedInitializerCoercion::ListToCombo { .. } => {
            format!("::iced::widget::combo_box::State::new({code})")
        }
        CheckedInitializerCoercion::ValueToAnimation { value } => {
            let code = if *value == Type::F64 {
                format!("({code}) as f32")
            } else {
                code
            };
            format!("::iced::Animation::new({code})")
        }
        CheckedInitializerCoercion::StrToMarkdown => {
            match &facts.expression(expression_use.root).kind {
                CheckedExprKind::Str(value) => format!(
                    "::iced::widget::markdown::Content::parse({})",
                    rust_string(value)
                ),
                _ => format!("::iced::widget::markdown::Content::parse(&({code}))"),
            }
        }
        CheckedInitializerCoercion::StrToEditor => {
            match &facts.expression(expression_use.root).kind {
                CheckedExprKind::Str(value) => format!(
                    "::iced::widget::text_editor::Content::with_text({})",
                    rust_string(value)
                ),
                _ => format!("::iced::widget::text_editor::Content::with_text(&({code}))"),
            }
        }
    })
}

pub(in crate::codegen) fn checked_expr_node_code(
    program: &LoweredProgram,
    expression_use: CheckedExprUseId,
    node: CheckedExprId,
    env: &dyn BindingEnvironment,
    mode: ValueMode,
) -> Result<String, Error> {
    let expression = program
        .checked_facts()
        .try_expression(node)
        .ok_or_else(|| {
            Error::new(
                "E196",
                &Span::line(1),
                "checked test expression node is outside its arena",
            )
        })?;
    if expression.owner != expression_use {
        return Err(Error::new(
            "E196",
            &Span::line(1),
            "checked test expression node belongs to another expression use",
        ));
    }
    expr_node_code(
        ExprNode::Checked(node),
        env,
        &ExprEmission::for_checked(program),
        mode,
    )
}

mod binding;
mod children;
mod discovery;
mod routes;

pub(super) use binding::*;
pub(super) use children::*;
pub(super) use discovery::*;
pub(super) use routes::*;
