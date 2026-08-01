use super::expr::{
    BuiltinArgumentContext, ContextualBuiltin, ExprTypeAnalysis, analyze_expr_types, field_type,
};
use super::*;
use crate::unqualified_name;
#[cfg(test)]
use std::cell::Cell;
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CheckedExprId(u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CheckedExprUseId(u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CheckedValueId(u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CheckedLocalId(u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CheckedViewId(u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CheckedOriginId(u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CheckedBuiltinId(u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CheckedExternFnId(u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CheckedStructId(u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CheckedStructFieldId {
    owner: CheckedStructId,
    index: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CheckedEnumId(u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CheckedEnumVariantId {
    owner: CheckedEnumId,
    index: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CheckedPaletteId(u32);

#[derive(Clone, Debug)]
pub(crate) struct CheckedOrigin {
    pub(crate) path: Option<PathBuf>,
    pub(crate) line: usize,
    pub(crate) column: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum ValueScope {
    App,
    Component(u32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CheckedValueKind {
    AppState { index: u32 },
    Derived { index: u32 },
    ComponentParam { component: u32, index: u32 },
    ComponentState { component: u32, index: u32 },
}

#[derive(Clone, Debug)]
pub(crate) struct CheckedValue {
    pub(crate) name: String,
    pub(crate) ty: Type,
    pub(crate) kind: CheckedValueKind,
    pub(crate) initializer: Option<CheckedExprUseId>,
    pub(crate) origin: CheckedOriginId,
}

#[derive(Clone, Debug)]
pub(crate) struct CheckedLocal {
    pub(crate) name: String,
    pub(crate) ty: Type,
    pub(crate) owner: CheckedExprUseId,
    pub(crate) body_argument: usize,
    pub(crate) origin: CheckedOriginId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CheckedViewScope {
    App,
    Component(u32),
    Test(u32),
}

#[derive(Clone, Debug)]
pub(crate) struct CheckedView {
    pub(crate) kind: &'static str,
    pub(crate) scope: CheckedViewScope,
    pub(crate) parent: Option<CheckedViewId>,
    pub(crate) children: Vec<CheckedViewId>,
    pub(crate) origin: CheckedOriginId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CheckedExprOwner {
    Value(CheckedValueId),
}

#[derive(Clone, Debug)]
pub(crate) struct CheckedExprUse {
    pub(crate) owner: CheckedExprOwner,
    pub(crate) root: CheckedExprId,
    pub(crate) expected: Type,
    pub(crate) origin: CheckedOriginId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum CheckedPathRoot {
    Value(CheckedValueId),
    Local(CheckedLocalId),
    EnumVariant(CheckedEnumVariantId),
    Palette(CheckedPaletteId),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum CheckedProjectionKind {
    Struct(CheckedStructFieldId),
    Native,
    OptionalWidgetTarget,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CheckedProjection {
    pub(crate) field: String,
    pub(crate) input: Type,
    pub(crate) output: Type,
    pub(crate) kind: CheckedProjectionKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum CheckedCallTarget {
    Builtin(CheckedBuiltinId),
    Extern(CheckedExternFnId),
    EnumVariant(CheckedEnumVariantId),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum CheckedCallArgument {
    Value(CheckedExprId),
    Binding(CheckedLocalId),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum CheckedUnaryOperator {
    BooleanNot,
    NumericNegation(Type),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum CheckedBinaryOperator {
    Boolean(BinaryOp),
    Equality { op: BinaryOp, operand: Type },
    Ordering { op: BinaryOp, operand: Type },
    Arithmetic { op: BinaryOp, operand: Type },
}

#[derive(Clone, Debug)]
pub(crate) enum CheckedExprKind {
    Bool(bool),
    I64(i64),
    F64(f64),
    Str(String),
    Bytes(Vec<u8>),
    List(Vec<CheckedExprId>),
    None,
    Path {
        root: CheckedPathRoot,
        projections: Vec<CheckedProjection>,
    },
    Call {
        target: CheckedCallTarget,
        arguments: Vec<CheckedCallArgument>,
    },
    Unary {
        operator: CheckedUnaryOperator,
        value: CheckedExprId,
    },
    Binary {
        operator: CheckedBinaryOperator,
        left: CheckedExprId,
        right: CheckedExprId,
    },
}

#[derive(Clone, Debug)]
pub(crate) struct CheckedExpr {
    pub(crate) ty: Type,
    pub(crate) kind: CheckedExprKind,
    pub(crate) origin: CheckedOriginId,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct CheckedFactMetrics {
    pub(crate) values: usize,
    pub(crate) locals: usize,
    pub(crate) views: usize,
    pub(crate) expression_uses: usize,
    pub(crate) expressions: usize,
    pub(crate) type_analysis_queries: usize,
    pub(crate) type_analysis_nodes: usize,
    pub(crate) type_analysis_cache_hits: usize,
    pub(crate) declaration_lookups: usize,
    pub(crate) builtin_intern_lookups: usize,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct CheckedFacts {
    values: Vec<CheckedValue>,
    locals: Vec<CheckedLocal>,
    views: Vec<CheckedView>,
    expression_uses: Vec<CheckedExprUse>,
    expressions: Vec<CheckedExpr>,
    builtins: Vec<String>,
    origins: Vec<CheckedOrigin>,
    metrics: CheckedFactMetrics,
    #[cfg(test)]
    lookup_count: Cell<usize>,
}

impl CheckedFacts {
    pub(crate) fn values(&self) -> &[CheckedValue] {
        &self.values
    }

    pub(crate) fn value(&self, id: CheckedValueId) -> &CheckedValue {
        self.record_lookup();
        &self.values[id.0 as usize]
    }

    pub(crate) fn locals(&self) -> &[CheckedLocal] {
        &self.locals
    }

    pub(crate) fn local(&self, id: CheckedLocalId) -> &CheckedLocal {
        self.record_lookup();
        &self.locals[id.0 as usize]
    }

    pub(crate) fn views(&self) -> &[CheckedView] {
        &self.views
    }

    pub(crate) fn view(&self, id: CheckedViewId) -> &CheckedView {
        self.record_lookup();
        &self.views[id.0 as usize]
    }

    pub(crate) fn expression_use(&self, id: CheckedExprUseId) -> &CheckedExprUse {
        self.record_lookup();
        &self.expression_uses[id.0 as usize]
    }

    pub(crate) fn expression(&self, id: CheckedExprId) -> &CheckedExpr {
        self.record_lookup();
        &self.expressions[id.0 as usize]
    }

    pub(crate) fn builtin(&self, id: CheckedBuiltinId) -> &str {
        self.record_lookup();
        &self.builtins[id.0 as usize]
    }

    pub(crate) fn origin(&self, id: CheckedOriginId) -> &CheckedOrigin {
        self.record_lookup();
        &self.origins[id.0 as usize]
    }

    pub(crate) fn metrics(&self) -> CheckedFactMetrics {
        self.metrics
    }

    #[cfg(test)]
    pub(crate) fn lookup_count(&self) -> usize {
        self.lookup_count.get()
    }

    #[cfg(test)]
    pub(crate) fn reset_lookup_count(&self) {
        self.lookup_count.set(0);
    }

    #[cfg(test)]
    fn record_lookup(&self) {
        self.lookup_count.set(self.lookup_count.get() + 1);
    }

    #[cfg(not(test))]
    fn record_lookup(&self) {}

    pub(crate) fn remap_origins(&mut self, origins: &[(PathBuf, usize)]) {
        for origin in &mut self.origins {
            let Some((path, line)) = origin
                .line
                .checked_sub(1)
                .and_then(|index| origins.get(index))
            else {
                continue;
            };
            origin.path = Some(path.clone());
            origin.line = *line;
        }
    }
}

pub(in crate::check) fn build(document: &Document) -> Result<CheckedFacts, Error> {
    FactsBuilder::new(document).build()
}

struct FactsBuilder<'a> {
    document: &'a Document,
    facts: CheckedFacts,
    values_by_scope: HashMap<ValueScope, HashMap<String, CheckedValueId>>,
    externs_by_name: HashMap<String, CheckedExternFnId>,
    structs_by_name: HashMap<String, CheckedStructId>,
    struct_fields_by_owner: HashMap<CheckedStructId, HashMap<String, CheckedStructFieldId>>,
    enums_by_name: HashMap<String, CheckedEnumId>,
    enum_variants_by_owner: HashMap<CheckedEnumId, HashMap<String, CheckedEnumVariantId>>,
    palettes_by_name: HashMap<String, CheckedPaletteId>,
    builtins_by_name: HashMap<String, CheckedBuiltinId>,
}

type FactEnv = HashMap<String, (CheckedPathRoot, Type)>;

#[derive(Clone, Copy)]
struct ExpressionLowering<'a> {
    analysis: &'a ExprTypeAnalysis,
    owner: CheckedExprUseId,
    origin: CheckedOriginId,
    span: &'a Span,
}

impl<'a> FactsBuilder<'a> {
    fn new(document: &'a Document) -> Self {
        Self {
            document,
            facts: CheckedFacts::default(),
            values_by_scope: HashMap::new(),
            externs_by_name: document
                .functions
                .iter()
                .enumerate()
                .filter(|(_, function)| function.kind == ExternKind::Sync)
                .map(|(index, function)| (function.name.clone(), CheckedExternFnId(index as u32)))
                .collect(),
            structs_by_name: document
                .structs
                .iter()
                .enumerate()
                .map(|(index, item)| (item.name.clone(), CheckedStructId(index as u32)))
                .collect(),
            struct_fields_by_owner: document
                .structs
                .iter()
                .enumerate()
                .map(|(struct_index, item)| {
                    let owner = CheckedStructId(struct_index as u32);
                    let fields = item
                        .fields
                        .iter()
                        .enumerate()
                        .map(|(index, (name, _))| {
                            (
                                name.clone(),
                                CheckedStructFieldId {
                                    owner,
                                    index: index as u32,
                                },
                            )
                        })
                        .collect();
                    (owner, fields)
                })
                .collect(),
            enums_by_name: document
                .enums
                .iter()
                .enumerate()
                .map(|(index, item)| (item.name.clone(), CheckedEnumId(index as u32)))
                .collect(),
            enum_variants_by_owner: document
                .enums
                .iter()
                .enumerate()
                .map(|(enum_index, item)| {
                    let owner = CheckedEnumId(enum_index as u32);
                    let variants = item
                        .variants
                        .iter()
                        .enumerate()
                        .map(|(index, variant)| {
                            (
                                variant.name.clone(),
                                CheckedEnumVariantId {
                                    owner,
                                    index: index as u32,
                                },
                            )
                        })
                        .collect();
                    (owner, variants)
                })
                .collect(),
            palettes_by_name: document
                .palettes
                .iter()
                .enumerate()
                .map(|(index, item)| (item.name.clone(), CheckedPaletteId(index as u32)))
                .collect(),
            builtins_by_name: HashMap::new(),
        }
    }

    fn build(mut self) -> Result<CheckedFacts, Error> {
        self.index_values()?;
        self.lower_initializers()?;
        self.index_views();
        if let Some(expression) = self
            .facts
            .expressions
            .iter()
            .find(|expression| contains_unknown(&expression.ty))
        {
            let origin = &self.facts.origins[expression.origin.0 as usize];
            return Err(self.invariant(
                &Span {
                    line: origin.line,
                    column: origin.column,
                },
                "checked expression fact retained an unresolved type",
            ));
        }
        self.facts.metrics.values = self.facts.values.len();
        self.facts.metrics.locals = self.facts.locals.len();
        self.facts.metrics.views = self.facts.views.len();
        self.facts.metrics.expression_uses = self.facts.expression_uses.len();
        self.facts.metrics.expressions = self.facts.expressions.len();
        Ok(self.facts)
    }

    fn index_values(&mut self) -> Result<(), Error> {
        for (index, state) in self.document.states.iter().enumerate() {
            self.push_value(
                ValueScope::App,
                state.name.clone(),
                state.ty.clone(),
                CheckedValueKind::AppState {
                    index: index as u32,
                },
                &state.span,
            )?;
        }
        for (index, derived) in self.document.derived.iter().enumerate() {
            self.push_value(
                ValueScope::App,
                derived.name.clone(),
                derived.ty.clone(),
                CheckedValueKind::Derived {
                    index: index as u32,
                },
                &derived.span,
            )?;
        }
        for (component_index, component) in self.document.components.iter().enumerate() {
            let scope = ValueScope::Component(component_index as u32);
            for (index, param) in component.params.iter().enumerate() {
                self.push_value(
                    scope,
                    param.name.clone(),
                    param.ty.clone(),
                    CheckedValueKind::ComponentParam {
                        component: component_index as u32,
                        index: index as u32,
                    },
                    &component.span,
                )?;
            }
            for (index, state) in component.states.iter().enumerate() {
                self.push_value(
                    scope,
                    state.name.clone(),
                    state.ty.clone(),
                    CheckedValueKind::ComponentState {
                        component: component_index as u32,
                        index: index as u32,
                    },
                    &state.span,
                )?;
            }
        }
        Ok(())
    }

    fn push_value(
        &mut self,
        scope: ValueScope,
        name: String,
        ty: Type,
        kind: CheckedValueKind,
        span: &Span,
    ) -> Result<CheckedValueId, Error> {
        let id = CheckedValueId(self.facts.values.len() as u32);
        if self
            .values_by_scope
            .entry(scope)
            .or_default()
            .insert(name.clone(), id)
            .is_some()
        {
            return Err(self.invariant(span, format!("duplicate checked value `{name}`")));
        }
        let origin = self.push_origin(span);
        self.facts.values.push(CheckedValue {
            name,
            ty,
            kind,
            initializer: None,
            origin,
        });
        Ok(id)
    }

    fn lower_initializers(&mut self) -> Result<(), Error> {
        let app_env = self
            .values_by_scope
            .get(&ValueScope::App)
            .into_iter()
            .flatten()
            .map(|(name, id)| {
                (
                    name.clone(),
                    (
                        CheckedPathRoot::Value(*id),
                        self.facts.values[id.0 as usize].ty.clone(),
                    ),
                )
            })
            .collect::<HashMap<_, _>>();

        for (index, state) in self.document.states.iter().enumerate() {
            let id = self.value_id(ValueScope::App, &state.name, &state.span)?;
            self.push_expression_use(id, &state.initial, &state.ty, &HashMap::new(), &state.span)?;
            debug_assert_eq!(id, CheckedValueId(index as u32));
        }
        for derived in &self.document.derived {
            let id = self.value_id(ValueScope::App, &derived.name, &derived.span)?;
            self.push_expression_use(id, &derived.value, &derived.ty, &app_env, &derived.span)?;
        }
        for (component_index, component) in self.document.components.iter().enumerate() {
            let scope = ValueScope::Component(component_index as u32);
            for param in &component.params {
                let Some(default) = &param.default else {
                    continue;
                };
                let id = self.value_id(scope, &param.name, &component.span)?;
                self.push_expression_use(id, default, &param.ty, &HashMap::new(), &component.span)?;
            }
            for state in &component.states {
                let id = self.value_id(scope, &state.name, &state.span)?;
                self.push_expression_use(
                    id,
                    &state.initial,
                    &state.ty,
                    &HashMap::new(),
                    &state.span,
                )?;
            }
        }
        Ok(())
    }

    fn value_id(
        &mut self,
        scope: ValueScope,
        name: &str,
        span: &Span,
    ) -> Result<CheckedValueId, Error> {
        self.facts.metrics.declaration_lookups += 1;
        self.values_by_scope
            .get(&scope)
            .and_then(|values| values.get(name))
            .copied()
            .ok_or_else(|| self.invariant(span, format!("missing checked value `{name}`")))
    }

    fn push_expression_use(
        &mut self,
        owner: CheckedValueId,
        expr: &Expr,
        expected: &Type,
        env: &FactEnv,
        span: &Span,
    ) -> Result<CheckedExprUseId, Error> {
        let origin = self.facts.values[owner.0 as usize].origin;
        let env_types = env
            .iter()
            .map(|(name, (_, ty))| (name.clone(), ty.clone()))
            .collect::<HashMap<_, _>>();
        let id = CheckedExprUseId(self.facts.expression_uses.len() as u32);
        let analysis = analyze_expr_types(expr, &env_types, self.document, span)?;
        let analysis_metrics = analysis.metrics();
        self.facts.metrics.type_analysis_queries += analysis_metrics.queries;
        self.facts.metrics.type_analysis_nodes += analysis_metrics.nodes;
        self.facts.metrics.type_analysis_cache_hits += analysis_metrics.cache_hits;
        let lowering = ExpressionLowering {
            analysis: &analysis,
            owner: id,
            origin,
            span,
        };
        let root = self.lower_expr(expr, Some(expected), env, lowering)?;
        debug_assert_eq!(id.0 as usize, self.facts.expression_uses.len());
        self.facts.expression_uses.push(CheckedExprUse {
            owner: CheckedExprOwner::Value(owner),
            root,
            expected: expected.clone(),
            origin,
        });
        self.facts.values[owner.0 as usize].initializer = Some(id);
        Ok(id)
    }

    fn lower_expr(
        &mut self,
        expr: &Expr,
        expected: Option<&Type>,
        env: &FactEnv,
        lowering: ExpressionLowering<'_>,
    ) -> Result<CheckedExprId, Error> {
        let inferred =
            lowering.analysis.type_of(expr).cloned().ok_or_else(|| {
                self.invariant(lowering.span, "missing post-order expression type")
            })?;
        let ty = contextual_type(inferred, expected);
        let kind =
            match expr {
                Expr::Bool(value) => CheckedExprKind::Bool(*value),
                Expr::I64(value) => CheckedExprKind::I64(*value),
                Expr::F64(value) => CheckedExprKind::F64(*value),
                Expr::Str(value) => CheckedExprKind::Str(value.clone()),
                Expr::Bytes(value) => CheckedExprKind::Bytes(value.clone()),
                Expr::EmptyList => CheckedExprKind::List(Vec::new()),
                Expr::List(values) => {
                    let expected_element = match &ty {
                        Type::List(inner) => Some(inner.as_ref()),
                        _ => None,
                    };
                    let children = values
                        .iter()
                        .map(|value| self.lower_expr(value, expected_element, env, lowering))
                        .collect::<Result<Vec<_>, _>>()?;
                    CheckedExprKind::List(children)
                }
                Expr::None => CheckedExprKind::None,
                Expr::Path(path) => self.lower_path(path, env, lowering.span)?,
                Expr::Call { name, args } => {
                    let target = self.resolve_call_target(name, lowering.span)?;
                    let arguments = self.lower_call_arguments(&target, args, &ty, env, lowering)?;
                    CheckedExprKind::Call { target, arguments }
                }
                Expr::Unary { op, value } => {
                    let input = lowering.analysis.type_of(value).cloned().ok_or_else(|| {
                        self.invariant(lowering.span, "missing unary operand type")
                    })?;
                    let operator = match op {
                        UnaryOp::Not => CheckedUnaryOperator::BooleanNot,
                        UnaryOp::Neg => CheckedUnaryOperator::NumericNegation(input.clone()),
                    };
                    let value = self.lower_expr(value, Some(&input), env, lowering)?;
                    CheckedExprKind::Unary { operator, value }
                }
                Expr::Binary { left, op, right } => {
                    let left_ty = lowering.analysis.type_of(left).cloned().ok_or_else(|| {
                        self.invariant(lowering.span, "missing left operand type")
                    })?;
                    let right_ty = lowering.analysis.type_of(right).cloned().ok_or_else(|| {
                        self.invariant(lowering.span, "missing right operand type")
                    })?;
                    let operator = match op {
                        BinaryOp::And | BinaryOp::Or => CheckedBinaryOperator::Boolean(*op),
                        BinaryOp::Eq | BinaryOp::NotEq => CheckedBinaryOperator::Equality {
                            op: *op,
                            operand: compatible_operand(&left_ty, &right_ty),
                        },
                        BinaryOp::Lt | BinaryOp::LtEq | BinaryOp::Gt | BinaryOp::GtEq => {
                            CheckedBinaryOperator::Ordering {
                                op: *op,
                                operand: compatible_operand(&left_ty, &right_ty),
                            }
                        }
                        BinaryOp::Add
                        | BinaryOp::Sub
                        | BinaryOp::Mul
                        | BinaryOp::Div
                        | BinaryOp::Rem => CheckedBinaryOperator::Arithmetic {
                            op: *op,
                            operand: compatible_operand(&left_ty, &right_ty),
                        },
                    };
                    let operand = match &operator {
                        CheckedBinaryOperator::Boolean(_) => Type::Bool,
                        CheckedBinaryOperator::Equality { operand, .. }
                        | CheckedBinaryOperator::Ordering { operand, .. }
                        | CheckedBinaryOperator::Arithmetic { operand, .. } => operand.clone(),
                    };
                    let left = self.lower_expr(left, Some(&operand), env, lowering)?;
                    let right = self.lower_expr(right, Some(&operand), env, lowering)?;
                    CheckedExprKind::Binary {
                        operator,
                        left,
                        right,
                    }
                }
            };
        let id = CheckedExprId(self.facts.expressions.len() as u32);
        self.facts.expressions.push(CheckedExpr {
            ty,
            kind,
            origin: lowering.origin,
        });
        Ok(id)
    }

    fn lower_path(
        &mut self,
        path: &[String],
        env: &FactEnv,
        span: &Span,
    ) -> Result<CheckedExprKind, Error> {
        if let [contract, palette] = path
            && self
                .document
                .theme_contract
                .as_ref()
                .is_some_and(|item| item.name == *contract)
        {
            self.facts.metrics.declaration_lookups += 1;
            let id = self.palettes_by_name.get(palette).copied().ok_or_else(|| {
                self.invariant(span, format!("missing checked palette `{palette}`"))
            })?;
            return Ok(CheckedExprKind::Path {
                root: CheckedPathRoot::Palette(id),
                projections: Vec::new(),
            });
        }
        if let [enum_name, variant_name] = path
            && let Some(variant) = self.enum_variant(enum_name, variant_name)
        {
            return Ok(CheckedExprKind::Path {
                root: CheckedPathRoot::EnumVariant(variant),
                projections: Vec::new(),
            });
        }
        let Some(name) = path.first() else {
            return Err(self.invariant(span, "checked path is empty"));
        };
        self.facts.metrics.declaration_lookups += 1;
        let (root, mut input) = env
            .get(name)
            .cloned()
            .ok_or_else(|| self.invariant(span, format!("missing checked value `{name}`")))?;
        let mut projections = Vec::with_capacity(path.len().saturating_sub(1));
        for field in &path[1..] {
            let output = field_type(&input, field, self.document, span)?;
            let kind = if let Type::Named(struct_name) = &input {
                self.facts.metrics.declaration_lookups += 1;
                let owner = self
                    .structs_by_name
                    .get(struct_name)
                    .copied()
                    .ok_or_else(|| {
                        self.invariant(span, format!("missing checked struct `{struct_name}`"))
                    })?;
                let field_id = self
                    .struct_fields_by_owner
                    .get(&owner)
                    .and_then(|fields| fields.get(field))
                    .copied()
                    .ok_or_else(|| {
                        self.invariant(
                            span,
                            format!("missing checked field `{struct_name}.{field}`"),
                        )
                    })?;
                CheckedProjectionKind::Struct(field_id)
            } else if matches!(&input, Type::Option(inner) if **inner == Type::WidgetTarget) {
                CheckedProjectionKind::OptionalWidgetTarget
            } else {
                CheckedProjectionKind::Native
            };
            projections.push(CheckedProjection {
                field: field.clone(),
                input,
                output: output.clone(),
                kind,
            });
            input = output;
        }
        Ok(CheckedExprKind::Path { root, projections })
    }

    fn resolve_call_target(&mut self, name: &str, span: &Span) -> Result<CheckedCallTarget, Error> {
        if let Some((enum_name, variant_name)) = name.split_once('.')
            && let Some(variant) = self.enum_variant(enum_name, variant_name)
        {
            return Ok(CheckedCallTarget::EnumVariant(variant));
        }
        self.facts.metrics.declaration_lookups += 1;
        if let Some(id) = self.externs_by_name.get(name).copied() {
            return Ok(CheckedCallTarget::Extern(id));
        }
        let name = unqualified_name(name);
        if name == "provided" {
            return Err(self.invariant(
                span,
                "provided slot facts belong to the future view-expression slice",
            ));
        }
        self.facts.metrics.builtin_intern_lookups += 1;
        let id = if let Some(id) = self.builtins_by_name.get(name).copied() {
            id
        } else {
            let id = CheckedBuiltinId(self.facts.builtins.len() as u32);
            self.facts.builtins.push(name.to_owned());
            self.builtins_by_name.insert(name.to_owned(), id);
            id
        };
        Ok(CheckedCallTarget::Builtin(id))
    }

    fn enum_variant(
        &mut self,
        enum_name: &str,
        variant_name: &str,
    ) -> Option<CheckedEnumVariantId> {
        self.facts.metrics.declaration_lookups += 1;
        let owner = self.enums_by_name.get(enum_name).copied()?;
        self.enum_variants_by_owner
            .get(&owner)?
            .get(variant_name)
            .copied()
    }

    fn lower_call_arguments(
        &mut self,
        target: &CheckedCallTarget,
        args: &[Expr],
        output: &Type,
        env: &FactEnv,
        lowering: ExpressionLowering<'_>,
    ) -> Result<Vec<CheckedCallArgument>, Error> {
        let contexts =
            self.call_argument_contexts(target, output, args, lowering.analysis, lowering.span)?;
        if contexts.len() != args.len() {
            return Err(self.invariant(
                lowering.span,
                "checked call argument context count does not match its arguments",
            ));
        }
        let mut bindings = HashMap::<usize, (String, CheckedLocalId)>::new();
        let mut arguments = Vec::with_capacity(args.len());
        for (index, (argument, context)) in args.iter().zip(contexts).enumerate() {
            match context {
                BuiltinArgumentContext::Value { expected } => {
                    arguments.push(CheckedCallArgument::Value(self.lower_expr(
                        argument,
                        expected.as_ref(),
                        env,
                        lowering,
                    )?));
                }
                BuiltinArgumentContext::Binding { ty, body } => {
                    let Expr::Path(path) = argument else {
                        return Err(
                            self.invariant(lowering.span, "checked builtin binding is not a path")
                        );
                    };
                    let [name] = path.as_slice() else {
                        return Err(self.invariant(
                            lowering.span,
                            "checked builtin binding is not a local name",
                        ));
                    };
                    let id = CheckedLocalId(self.facts.locals.len() as u32);
                    self.facts.locals.push(CheckedLocal {
                        name: name.clone(),
                        ty,
                        owner: lowering.owner,
                        body_argument: body,
                        origin: lowering.origin,
                    });
                    bindings.insert(index, (name.clone(), id));
                    arguments.push(CheckedCallArgument::Binding(id));
                }
                BuiltinArgumentContext::ScopedValue { expected, binding } => {
                    let (name, local) = bindings.get(&binding).cloned().ok_or_else(|| {
                        self.invariant(lowering.span, "checked builtin body has no binding fact")
                    })?;
                    let mut scoped = env.clone();
                    let ty = self.facts.locals[local.0 as usize].ty.clone();
                    scoped.insert(name, (CheckedPathRoot::Local(local), ty));
                    arguments.push(CheckedCallArgument::Value(self.lower_expr(
                        argument,
                        expected.as_ref(),
                        &scoped,
                        lowering,
                    )?));
                }
            }
        }
        Ok(arguments)
    }

    fn call_argument_contexts(
        &self,
        target: &CheckedCallTarget,
        output: &Type,
        args: &[Expr],
        analysis: &ExprTypeAnalysis,
        span: &Span,
    ) -> Result<Vec<BuiltinArgumentContext>, Error> {
        Ok(match target {
            CheckedCallTarget::Extern(id) => self.document.functions[id.0 as usize]
                .params
                .iter()
                .map(|(_, ty)| BuiltinArgumentContext::Value {
                    expected: Some(ty.clone()),
                })
                .collect(),
            CheckedCallTarget::EnumVariant(id) => self.document.enums[id.owner.0 as usize].variants
                [id.index as usize]
                .payload
                .iter()
                .cloned()
                .map(|ty| BuiltinArgumentContext::Value { expected: Some(ty) })
                .collect(),
            CheckedCallTarget::Builtin(id) => {
                let name = self.facts.builtins[id.0 as usize].as_str();
                if let Some(builtin) = ContextualBuiltin::from_name(name) {
                    let inferred = args
                        .iter()
                        .map(|argument| {
                            analysis.type_of(argument).cloned().unwrap_or(Type::Unknown)
                        })
                        .collect::<Vec<_>>();
                    builtin
                        .argument_contexts(output, &inferred)
                        .map_err(|message| self.invariant(span, message))?
                } else {
                    args.iter()
                        .map(|_| BuiltinArgumentContext::Value { expected: None })
                        .collect()
                }
            }
        })
    }

    fn index_views(&mut self) {
        for (index, component) in self.document.components.iter().enumerate() {
            self.index_view(
                &component.root,
                None,
                CheckedViewScope::Component(index as u32),
            );
        }
        self.index_view(&self.document.view, None, CheckedViewScope::App);
        for (index, test) in self.document.tests.iter().enumerate() {
            if let Some(mount) = &test.mount {
                self.index_view(mount, None, CheckedViewScope::Test(index as u32));
            }
        }
    }

    fn index_view(
        &mut self,
        node: &ViewNode,
        parent: Option<CheckedViewId>,
        scope: CheckedViewScope,
    ) -> CheckedViewId {
        let id = CheckedViewId(self.facts.views.len() as u32);
        let origin = self.push_origin(node.span());
        self.facts.views.push(CheckedView {
            kind: view_kind(node),
            scope,
            parent,
            children: Vec::new(),
            origin,
        });
        let children = view_children(node)
            .into_iter()
            .map(|child| self.index_view(child, Some(id), scope))
            .collect();
        self.facts.views[id.0 as usize].children = children;
        id
    }

    fn push_origin(&mut self, span: &Span) -> CheckedOriginId {
        let id = CheckedOriginId(self.facts.origins.len() as u32);
        self.facts.origins.push(CheckedOrigin {
            path: None,
            line: span.line,
            column: span.column,
        });
        id
    }

    fn invariant(&self, span: &Span, message: impl Into<String>) -> Error {
        Error::new("E196", span, message)
    }
}

fn contextual_type(inferred: Type, expected: Option<&Type>) -> Type {
    match (inferred, expected) {
        (Type::Unknown, Some(expected)) => expected.clone(),
        (Type::List(actual), Some(Type::List(expected))) => {
            Type::List(Box::new(contextual_type(*actual, Some(expected.as_ref()))))
        }
        (Type::Option(actual), Some(Type::Option(expected))) => {
            Type::Option(Box::new(contextual_type(*actual, Some(expected.as_ref()))))
        }
        (Type::Result(actual_output, actual_error), Some(Type::Result(output, error))) => {
            Type::Result(
                Box::new(contextual_type(*actual_output, Some(output.as_ref()))),
                Box::new(contextual_type(*actual_error, Some(error.as_ref()))),
            )
        }
        (Type::Combo(actual), Some(Type::Combo(expected))) => {
            Type::Combo(Box::new(contextual_type(*actual, Some(expected.as_ref()))))
        }
        (Type::Animation(actual), Some(Type::Animation(expected))) => {
            Type::Animation(Box::new(contextual_type(*actual, Some(expected.as_ref()))))
        }
        (actual, _) => actual,
    }
}

fn contains_unknown(ty: &Type) -> bool {
    match ty {
        Type::Unknown => true,
        Type::List(inner) | Type::Option(inner) | Type::Combo(inner) | Type::Animation(inner) => {
            contains_unknown(inner)
        }
        Type::Result(output, error) => contains_unknown(output) || contains_unknown(error),
        _ => false,
    }
}

fn compatible_operand(left: &Type, right: &Type) -> Type {
    if *left == Type::Unknown {
        right.clone()
    } else {
        left.clone()
    }
}

fn view_kind(node: &ViewNode) -> &'static str {
    match node {
        ViewNode::Layout { .. } => "layout",
        ViewNode::Container { .. } => "container",
        ViewNode::Overlay { .. } => "overlay",
        ViewNode::PaneGrid { .. } => "pane-grid",
        ViewNode::Text { .. } => "text",
        ViewNode::RichText { .. } => "rich-text",
        ViewNode::Input { .. } => "input",
        ViewNode::Button { .. } => "button",
        ViewNode::Checkbox { .. } => "checkbox",
        ViewNode::Toggler { .. } => "toggler",
        ViewNode::Slider { .. } => "slider",
        ViewNode::Progress { .. } => "progress",
        ViewNode::Radio { .. } => "radio",
        ViewNode::PickList { .. } => "pick-list",
        ViewNode::ComboBox { .. } => "combo-box",
        ViewNode::Rule { .. } => "rule",
        ViewNode::QrCode { .. } => "qr-code",
        ViewNode::Space { .. } => "space",
        ViewNode::If { .. } => "if",
        ViewNode::Match { .. } => "match",
        ViewNode::For { .. } => "for",
        ViewNode::KeyedColumn { .. } => "keyed-column",
        ViewNode::Lazy { .. } => "lazy",
        ViewNode::Markdown { .. } => "markdown",
        ViewNode::TextEditor { .. } => "text-editor",
        ViewNode::Table { .. } => "table",
        ViewNode::Component { .. } => "component",
        ViewNode::Slot { .. } => "slot",
        ViewNode::ExternComponent { .. } => "extern-component",
        ViewNode::Themer { .. } => "themer",
        ViewNode::Shader { .. } => "shader",
        ViewNode::Media { .. } => "media",
        ViewNode::Tooltip { .. } => "tooltip",
        ViewNode::MouseArea { .. } => "mouse-area",
        ViewNode::ResizeHandle { .. } => "resize-handle",
        ViewNode::Canvas { .. } => "canvas",
        ViewNode::Theme { .. } => "theme",
        ViewNode::Float { .. } => "float",
        ViewNode::Pin { .. } => "pin",
        ViewNode::Sensor { .. } => "sensor",
        ViewNode::Responsive { .. } => "responsive",
    }
}

fn view_children(node: &ViewNode) -> Vec<&ViewNode> {
    match node {
        ViewNode::Layout { children, .. }
        | ViewNode::If { children, .. }
        | ViewNode::For { children, .. } => children.iter().collect(),
        ViewNode::Match { arms, .. } => arms.iter().flat_map(|arm| arm.children.iter()).collect(),
        ViewNode::Button {
            content: Some(content),
            ..
        }
        | ViewNode::MouseArea { content, .. }
        | ViewNode::ResizeHandle { content, .. }
        | ViewNode::Container { content, .. }
        | ViewNode::Theme { content, .. }
        | ViewNode::Float { content, .. }
        | ViewNode::Pin { content, .. }
        | ViewNode::Sensor { content, .. }
        | ViewNode::KeyedColumn { child: content, .. }
        | ViewNode::Lazy { child: content, .. } => vec![content],
        ViewNode::Tooltip { content, tip, .. } => vec![content, tip],
        ViewNode::Overlay { content, layer, .. } => vec![content, layer],
        ViewNode::PaneGrid {
            panes, templates, ..
        } => panes
            .iter()
            .flat_map(PaneView::nodes)
            .chain(templates.iter().flat_map(|template| template.pane.nodes()))
            .collect(),
        ViewNode::Table { columns, .. } => columns
            .iter()
            .flat_map(|column| [&column.header, &column.cell])
            .collect(),
        ViewNode::Component { slots, .. } => {
            slots.iter().map(|slot| slot.content.as_ref()).collect()
        }
        ViewNode::Responsive { content, .. } => match content {
            ResponsiveContent::Breakpoint { narrow, wide, .. } => vec![narrow, wide],
            ResponsiveContent::Size { content, .. } => vec![content],
        },
        _ => Vec::new(),
    }
}

#[cfg(test)]
impl CheckedFacts {
    fn structural_snapshot(&self) -> String {
        use std::fmt::Write as _;

        let mut output = String::new();
        for (index, value) in self.values.iter().enumerate() {
            writeln!(
                output,
                "value v{index} {:?} {}:{:?} init={:?} origin=o{}",
                value.kind, value.name, value.ty, value.initializer, value.origin.0
            )
            .unwrap();
        }
        for (index, expression_use) in self.expression_uses.iter().enumerate() {
            writeln!(
                output,
                "use u{index} {:?} root=e{} expected={:?} origin=o{}",
                expression_use.owner,
                expression_use.root.0,
                expression_use.expected,
                expression_use.origin.0
            )
            .unwrap();
        }
        for (index, local) in self.locals.iter().enumerate() {
            writeln!(
                output,
                "local l{index} {}:{:?} owner=u{} body_arg={} origin=o{}",
                local.name, local.ty, local.owner.0, local.body_argument, local.origin.0
            )
            .unwrap();
        }
        for (index, expression) in self.expressions.iter().enumerate() {
            let kind = match &expression.kind {
                CheckedExprKind::Bool(value) => format!("bool {value}"),
                CheckedExprKind::I64(value) => format!("i64 {value}"),
                CheckedExprKind::F64(value) => format!("f64 {value}"),
                CheckedExprKind::Str(value) => format!("str {value:?}"),
                CheckedExprKind::Bytes(value) => format!("bytes {value:?}"),
                CheckedExprKind::List(values) => format!("list {values:?}"),
                CheckedExprKind::None => "none".into(),
                CheckedExprKind::Path { root, projections } => {
                    format!("path {root:?} {projections:?}")
                }
                CheckedExprKind::Call { target, arguments } => match target {
                    CheckedCallTarget::Builtin(id) => format!(
                        "call builtin:{} {}",
                        self.builtins[id.0 as usize],
                        format_call_arguments(arguments)
                    ),
                    _ => format!("call {target:?} {}", format_call_arguments(arguments)),
                },
                CheckedExprKind::Unary { operator, value } => {
                    format!("unary {operator:?} e{}", value.0)
                }
                CheckedExprKind::Binary {
                    operator,
                    left,
                    right,
                } => format!("binary {operator:?} e{} e{}", left.0, right.0),
            };
            writeln!(
                output,
                "expr e{index} {kind} : {:?} origin=o{}",
                expression.ty, expression.origin.0
            )
            .unwrap();
        }
        for (index, view) in self.views.iter().enumerate() {
            writeln!(
                output,
                "view w{index} {} {:?} parent={:?} children={:?} origin=o{}",
                view.kind, view.scope, view.parent, view.children, view.origin.0
            )
            .unwrap();
        }
        output
    }
}

#[cfg(test)]
fn format_call_arguments(arguments: &[CheckedCallArgument]) -> String {
    let values = arguments
        .iter()
        .map(|argument| match argument {
            CheckedCallArgument::Value(id) => format!("CheckedExprId({})", id.0),
            CheckedCallArgument::Binding(id) => format!("CheckedLocalId({})", id.0),
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!("[{values}]")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{analyze, analyze_file, lower};
    use std::fmt::Write as _;
    use std::fs;
    use std::time::{Instant, SystemTime, UNIX_EPOCH};

    const THEME: &str = "theme contract AppTheme\n  bg\n  fg\n  primary\n  danger\npalette app for AppTheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\n";

    #[test]
    fn snapshots_resolved_expression_and_owner_facts() {
        let source = format!(
            r#"app Facts
extern crate::backend
  User(name:str)
  sync load_user(seed:i64) -> User
enum Mode
  idle
  active(str)
{THEME}state
  user:User = load_user(1)
  color:color = color.rgb(0.25, 0.5, 0.75)
  mode:Mode = Mode.idle
derived
  name = user.name
  visible = color.r > 0.1 && name != ""
component Card(label:str="Card")
  state
    open = false
  col
    text label
    if open
      text "Open"
view
  col
    Card
    text name
"#
        );
        let program = lower::lower(analyze(&source).unwrap()).unwrap();
        let facts = program.checked_facts();

        assert_eq!(
            facts.structural_snapshot(),
            r#"value v0 AppState { index: 0 } user:Named("User") init=Some(CheckedExprUseId(0)) origin=o0
value v1 AppState { index: 1 } color:Color init=Some(CheckedExprUseId(1)) origin=o1
value v2 AppState { index: 2 } mode:Named("Mode") init=Some(CheckedExprUseId(2)) origin=o2
value v3 Derived { index: 0 } name:Str init=Some(CheckedExprUseId(3)) origin=o3
value v4 Derived { index: 1 } visible:Bool init=Some(CheckedExprUseId(4)) origin=o4
value v5 ComponentParam { component: 0, index: 0 } label:Str init=Some(CheckedExprUseId(5)) origin=o5
value v6 ComponentState { component: 0, index: 0 } open:Bool init=Some(CheckedExprUseId(6)) origin=o6
use u0 Value(CheckedValueId(0)) root=e1 expected=Named("User") origin=o0
use u1 Value(CheckedValueId(1)) root=e5 expected=Color origin=o1
use u2 Value(CheckedValueId(2)) root=e6 expected=Named("Mode") origin=o2
use u3 Value(CheckedValueId(3)) root=e7 expected=Str origin=o3
use u4 Value(CheckedValueId(4)) root=e14 expected=Bool origin=o4
use u5 Value(CheckedValueId(5)) root=e15 expected=Str origin=o5
use u6 Value(CheckedValueId(6)) root=e16 expected=Bool origin=o6
expr e0 i64 1 : I64 origin=o0
expr e1 call Extern(CheckedExternFnId(0)) [CheckedExprId(0)] : Named("User") origin=o0
expr e2 f64 0.25 : F64 origin=o1
expr e3 f64 0.5 : F64 origin=o1
expr e4 f64 0.75 : F64 origin=o1
expr e5 call builtin:color.rgb [CheckedExprId(2), CheckedExprId(3), CheckedExprId(4)] : Color origin=o1
expr e6 path EnumVariant(CheckedEnumVariantId { owner: CheckedEnumId(0), index: 0 }) [] : Named("Mode") origin=o2
expr e7 path Value(CheckedValueId(0)) [CheckedProjection { field: "name", input: Named("User"), output: Str, kind: Struct(CheckedStructFieldId { owner: CheckedStructId(0), index: 0 }) }] : Str origin=o3
expr e8 path Value(CheckedValueId(1)) [CheckedProjection { field: "r", input: Color, output: F64, kind: Native }] : F64 origin=o4
expr e9 f64 0.1 : F64 origin=o4
expr e10 binary Ordering { op: Gt, operand: F64 } e8 e9 : Bool origin=o4
expr e11 path Value(CheckedValueId(3)) [] : Str origin=o4
expr e12 str "" : Str origin=o4
expr e13 binary Equality { op: NotEq, operand: Str } e11 e12 : Bool origin=o4
expr e14 binary Boolean(And) e10 e13 : Bool origin=o4
expr e15 str "Card" : Str origin=o5
expr e16 bool false : Bool origin=o6
view w0 layout Component(0) parent=None children=[CheckedViewId(1), CheckedViewId(2)] origin=o7
view w1 text Component(0) parent=Some(CheckedViewId(0)) children=[] origin=o8
view w2 if Component(0) parent=Some(CheckedViewId(0)) children=[CheckedViewId(3)] origin=o9
view w3 text Component(0) parent=Some(CheckedViewId(2)) children=[] origin=o10
view w4 layout App parent=None children=[CheckedViewId(5), CheckedViewId(6)] origin=o11
view w5 component App parent=Some(CheckedViewId(4)) children=[] origin=o12
view w6 text App parent=Some(CheckedViewId(4)) children=[] origin=o13
"#
        );
        assert_eq!(
            facts.metrics(),
            CheckedFactMetrics {
                values: 7,
                locals: 0,
                views: 7,
                expression_uses: 7,
                expressions: 17,
                type_analysis_queries: 17,
                type_analysis_nodes: 17,
                type_analysis_cache_hits: 0,
                declaration_lookups: 17,
                builtin_intern_lookups: 1,
            }
        );
    }

    #[test]
    fn lowering_moves_checked_facts_without_re_resolving_the_ast() {
        let source = format!(
            "app Facts\n{THEME}state\n  value:color = color.black()\nview\n  text \"ok\"\n"
        );
        let mut checked = analyze(&source).unwrap();
        let before = checked.facts.structural_snapshot();
        checked.document.states[0].initial = Expr::Call {
            name: "color.white".into(),
            args: Vec::new(),
        };

        let program = lower::lower(checked).unwrap();
        assert_eq!(program.checked_facts().structural_snapshot(), before);
        let initializer = program.checked_facts().values()[0].initializer.unwrap();
        let root = program.checked_facts().expression_use(initializer).root;
        let CheckedExprKind::Call {
            target: CheckedCallTarget::Builtin(builtin),
            ..
        } = &program.checked_facts().expression(root).kind
        else {
            panic!("state initializer must remain a resolved builtin call");
        };
        assert_eq!(program.checked_facts().builtin(*builtin), "color.black");
    }

    #[test]
    fn imported_expression_origins_keep_their_physical_file() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "ui-lang-checked-facts-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&directory).unwrap();
        let root = directory.join("app.ice");
        let imported = directory.join("card.ice");
        fs::write(
            &root,
            format!("app Facts\nuse \"card.ice\" as ui\n{THEME}view\n  ui::Card\n"),
        )
        .unwrap();
        fs::write(
            &imported,
            "component Card()\n  state\n    open = false\n  text \"Card\"\n",
        )
        .unwrap();

        let program = lower::lower(analyze_file(&root).unwrap()).unwrap();
        let facts = program.checked_facts();
        let state = facts
            .values()
            .iter()
            .find(|value| matches!(value.kind, CheckedValueKind::ComponentState { .. }))
            .unwrap();
        let value_origin = facts.origin(state.origin);
        let initializer = facts.expression_use(state.initializer.unwrap());
        let expression_origin = facts.origin(facts.expression(initializer.root).origin);
        assert_eq!(value_origin.path.as_deref(), Some(imported.as_path()));
        assert_eq!(value_origin.line, 3);
        assert_eq!(value_origin.column, 1);
        assert_eq!(expression_origin.path.as_deref(), Some(imported.as_path()));
        assert_eq!(expression_origin.line, 3);

        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn animation_projection_records_a_typed_scoped_local() {
        let source = format!(
            "app Projection\n{THEME}state\n  progress:animation[f64] = 0.0\nderived\n  projected = animation.project(progress, sample, sample * 2.0)\nview\n  text projected\n"
        );

        let program = lower::lower(analyze(&source).unwrap()).unwrap();
        let facts = program.checked_facts();
        assert_eq!(facts.locals().len(), 1);
        let local = facts.local(CheckedLocalId(0));
        assert_eq!(local.name, "sample");
        assert_eq!(local.ty, Type::F64);
        assert_eq!(local.owner, CheckedExprUseId(1));
        assert_eq!(local.body_argument, 2);

        let projected = facts
            .values()
            .iter()
            .find(|value| value.name == "projected")
            .unwrap();
        let root = facts.expression_use(projected.initializer.unwrap()).root;
        let CheckedExprKind::Call { arguments, .. } = &facts.expression(root).kind else {
            panic!("projection initializer must be a checked call");
        };
        assert!(matches!(
            arguments.as_slice(),
            [
                CheckedCallArgument::Value(_),
                CheckedCallArgument::Binding(CheckedLocalId(0)),
                CheckedCallArgument::Value(_)
            ]
        ));
        let CheckedCallArgument::Value(body) = arguments[2] else {
            unreachable!();
        };
        let CheckedExprKind::Binary { left, .. } = facts.expression(body).kind else {
            panic!("projection body must retain its checked binary expression");
        };
        assert!(matches!(
            facts.expression(left).kind,
            CheckedExprKind::Path {
                root: CheckedPathRoot::Local(CheckedLocalId(0)),
                ..
            }
        ));
        assert_eq!(facts.metrics().locals, 1);
        assert_eq!(facts.metrics().type_analysis_nodes, 6);
        assert_eq!(facts.metrics().expressions, 6);
    }

    #[test]
    fn contextual_builtin_arguments_receive_the_declared_default_types() {
        let source = format!(
            "app Defaults\n{THEME}component Context(items:[str]=[], selected:str?=none, nested:str?=some(\"ready\"), success:result[str,str]=ok(\"yes\"), failure:result[str,str]=err(\"no\"))\n  text \"defaults\"\nview\n  Context\n"
        );

        let program = lower::lower(analyze(&source).unwrap()).unwrap();
        let facts = program.checked_facts();
        for (name, expected) in [
            ("items", Type::List(Box::new(Type::Str))),
            ("selected", Type::Option(Box::new(Type::Str))),
            ("nested", Type::Option(Box::new(Type::Str))),
            (
                "success",
                Type::Result(Box::new(Type::Str), Box::new(Type::Str)),
            ),
            (
                "failure",
                Type::Result(Box::new(Type::Str), Box::new(Type::Str)),
            ),
        ] {
            let value = facts
                .values()
                .iter()
                .find(|value| value.name == name)
                .unwrap();
            let expression =
                facts.expression(facts.expression_use(value.initializer.unwrap()).root);
            assert_eq!(expression.ty, expected, "{name}");
        }
        assert_eq!(
            facts.metrics().type_analysis_nodes,
            facts.metrics().expressions
        );
    }

    #[test]
    #[ignore = "explicit large checked-fact performance contract"]
    fn performance_contract_ten_thousand_fact_lookups_are_direct_arena_accesses() {
        const VALUES: usize = 10_000;
        let mut source = format!("app Facts\n{THEME}state\n");
        for index in 0..VALUES {
            writeln!(source, "  value_{index} = {index}").unwrap();
        }
        source.push_str("view\n  text \"ok\"\n");

        let started = Instant::now();
        let program = lower::lower(analyze(&source).unwrap()).unwrap();
        let elapsed = started.elapsed();
        let facts = program.checked_facts();
        assert_eq!(facts.metrics().values, VALUES);
        assert_eq!(facts.metrics().expressions, VALUES);
        assert_eq!(facts.metrics().type_analysis_queries, VALUES);
        assert_eq!(facts.metrics().type_analysis_nodes, VALUES);
        assert_eq!(facts.metrics().type_analysis_cache_hits, 0);
        assert_eq!(facts.metrics().declaration_lookups, VALUES);
        facts.reset_lookup_count();
        for index in 0..VALUES {
            let value = facts.value(CheckedValueId(index as u32));
            let expression_use = facts.expression_use(value.initializer.unwrap());
            assert_eq!(facts.expression(expression_use.root).ty, Type::I64);
        }
        for index in 0..facts.views().len() {
            assert_eq!(
                facts.view(CheckedViewId(index as u32)).scope,
                CheckedViewScope::App
            );
        }
        assert_eq!(facts.lookup_count(), VALUES * 3 + facts.views().len());
        assert!(
            elapsed.as_secs_f64() < 8.0,
            "10k checked value facts built and lowered in {elapsed:?}"
        );
    }

    #[test]
    #[ignore = "explicit deep linear checked-expression performance contract"]
    fn performance_contract_deep_expression_is_analyzed_once_per_node() {
        const TERMS: usize = 128;
        let (metrics, elapsed) = std::thread::Builder::new()
            .name("deep-expression-facts".into())
            .stack_size(64 * 1024 * 1024)
            .spawn(|| {
                let mut source = format!("app Deep\n{THEME}state\n  value:i64 = 1");
                for _ in 1..TERMS {
                    source.push_str(" + 1");
                }
                source.push_str("\nview\n  text value\n");
                let started = Instant::now();
                let program = lower::lower(analyze(&source).unwrap()).unwrap();
                (program.checked_facts().metrics(), started.elapsed())
            })
            .unwrap()
            .join()
            .unwrap();

        let nodes = TERMS * 2 - 1;
        assert_eq!(metrics.values, 1);
        assert_eq!(metrics.expression_uses, 1);
        assert_eq!(metrics.expressions, nodes);
        assert_eq!(metrics.type_analysis_queries, nodes);
        assert_eq!(metrics.type_analysis_nodes, nodes);
        assert_eq!(metrics.type_analysis_cache_hits, 0);
        assert!(
            elapsed.as_secs_f64() < 8.0,
            "128-term expression facts built and lowered in {elapsed:?}"
        );
    }
}
