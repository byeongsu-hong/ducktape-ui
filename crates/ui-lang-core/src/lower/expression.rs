use super::*;

pub(crate) use crate::hir::{
    ExpressionId as ResolvedExpressionId, ExpressionNodeId as ResolvedExpressionNodeId,
    LocalId as ResolvedLocalId, ValueRef as ResolvedValueRef,
};
#[derive(Clone, Debug)]
pub(crate) struct ResolvedValue {
    pub(crate) name: String,
    pub(crate) origin: OriginId,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedLocal {
    pub(crate) name: String,
    pub(crate) ty: Type,
    pub(crate) origin: OriginId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ResolvedInitializerCoercion {
    None,
    ListToCombo { element: Type },
    ValueToAnimation { value: Type },
    StrToMarkdown,
    StrToEditor,
}
#[derive(Clone, Debug)]
pub(crate) struct ResolvedExpressionUse {
    pub(crate) root: ResolvedExpressionNodeId,
    pub(crate) coercion: ResolvedInitializerCoercion,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ResolvedPathRoot {
    Value(ResolvedValueRef),
    Local(ResolvedLocalId),
    EnumVariant {
        enum_rust_name: String,
        variant_name: String,
    },
    Palette(PaletteId),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ResolvedProjectionKind {
    Struct(crate::hir::StructFieldId),
    Native,
    OptionalWidgetTarget,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ResolvedProjection {
    pub(crate) field: String,
    pub(crate) input: Type,
    pub(crate) output: Type,
    pub(crate) kind: ResolvedProjectionKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ResolvedCallTarget {
    Builtin(String),
    Extern(ExternFnId),
    EnumVariant {
        enum_rust_name: String,
        variant_name: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ResolvedCallArgument {
    Value(ResolvedExpressionNodeId),
    Binding(ResolvedLocalId),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ResolvedUnaryOperator {
    BooleanNot,
    NumericNegation,
}

#[derive(Clone, Debug)]
pub(crate) enum ResolvedExpressionKind {
    Bool(bool),
    I64(i64),
    F64(f64),
    Str(String),
    Bytes(Vec<u8>),
    List(Vec<ResolvedExpressionNodeId>),
    None,
    SlotProvided(ComponentSlotId),
    Path {
        root: ResolvedPathRoot,
        projections: Vec<ResolvedProjection>,
    },
    Call {
        target: ResolvedCallTarget,
        arguments: Vec<ResolvedCallArgument>,
    },
    Unary {
        operator: ResolvedUnaryOperator,
        value: ResolvedExpressionNodeId,
    },
    Binary {
        operator: BinaryOp,
        left: ResolvedExpressionNodeId,
        right: ResolvedExpressionNodeId,
    },
}
#[derive(Clone, Debug)]
pub(crate) struct ResolvedExpressionNode {
    pub(crate) owner: ResolvedExpressionId,
    pub(crate) ty: Type,
    pub(crate) kind: ResolvedExpressionKind,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedExpressionProgram {
    values: HashMap<ResolvedValueRef, ResolvedValue>,
    locals: Vec<ResolvedLocal>,
    uses: Vec<ResolvedExpressionUse>,
    nodes: Vec<ResolvedExpressionNode>,
    daemon_window_local: Option<ResolvedLocalId>,
}

impl ResolvedExpressionProgram {
    pub(super) fn from_checked(facts: &CheckedFacts, declarations: &DeclarationIndex) -> Self {
        let values = facts
            .values()
            .iter()
            .map(|value| {
                (
                    value.id,
                    ResolvedValue {
                        name: value.name.clone(),
                        origin: value.origin,
                    },
                )
            })
            .collect();
        let locals = facts
            .locals()
            .iter()
            .map(|local| ResolvedLocal {
                name: local.name.clone(),
                ty: local.ty.clone(),
                origin: local.origin,
            })
            .collect();
        let uses = facts
            .expression_uses()
            .iter()
            .map(|expression| ResolvedExpressionUse {
                root: expression.root,
                coercion: match &expression.coercion {
                    CheckedInitializerCoercion::None => ResolvedInitializerCoercion::None,
                    CheckedInitializerCoercion::ListToCombo { element } => {
                        ResolvedInitializerCoercion::ListToCombo {
                            element: element.clone(),
                        }
                    }
                    CheckedInitializerCoercion::ValueToAnimation { value } => {
                        ResolvedInitializerCoercion::ValueToAnimation {
                            value: value.clone(),
                        }
                    }
                    CheckedInitializerCoercion::StrToMarkdown => {
                        ResolvedInitializerCoercion::StrToMarkdown
                    }
                    CheckedInitializerCoercion::StrToEditor => {
                        ResolvedInitializerCoercion::StrToEditor
                    }
                },
            })
            .collect();
        let nodes = facts
            .expressions()
            .iter()
            .map(|expression| ResolvedExpressionNode {
                owner: expression.owner,
                ty: expression.ty.clone(),
                kind: match &expression.kind {
                    CheckedExprKind::Bool(value) => ResolvedExpressionKind::Bool(*value),
                    CheckedExprKind::I64(value) => ResolvedExpressionKind::I64(*value),
                    CheckedExprKind::F64(value) => ResolvedExpressionKind::F64(*value),
                    CheckedExprKind::Str(value) => ResolvedExpressionKind::Str(value.clone()),
                    CheckedExprKind::Bytes(values) => ResolvedExpressionKind::Bytes(values.clone()),
                    CheckedExprKind::List(values) => ResolvedExpressionKind::List(values.clone()),
                    CheckedExprKind::None => ResolvedExpressionKind::None,
                    CheckedExprKind::SlotProvided(slot) => {
                        ResolvedExpressionKind::SlotProvided(*slot)
                    }
                    CheckedExprKind::Path { root, projections } => ResolvedExpressionKind::Path {
                        root: match root {
                            CheckedPathRoot::Value(value) => ResolvedPathRoot::Value(*value),
                            CheckedPathRoot::Local(local) => ResolvedPathRoot::Local(*local),
                            CheckedPathRoot::EnumVariant(variant) => {
                                let declaration = declarations.enum_variant_decl(*variant);
                                let owner = declarations.enum_decl(variant.owner);
                                ResolvedPathRoot::EnumVariant {
                                    enum_rust_name: owner.rust_name.clone(),
                                    variant_name: declaration.name.clone(),
                                }
                            }
                            CheckedPathRoot::Palette(palette) => {
                                ResolvedPathRoot::Palette(*palette)
                            }
                        },
                        projections: projections
                            .iter()
                            .map(|projection| ResolvedProjection {
                                field: projection.field.clone(),
                                input: projection.input.clone(),
                                output: projection.output.clone(),
                                kind: match projection.kind {
                                    CheckedProjectionKind::Struct(field) => {
                                        ResolvedProjectionKind::Struct(field)
                                    }
                                    CheckedProjectionKind::Native => ResolvedProjectionKind::Native,
                                    CheckedProjectionKind::OptionalWidgetTarget => {
                                        ResolvedProjectionKind::OptionalWidgetTarget
                                    }
                                },
                            })
                            .collect(),
                    },
                    CheckedExprKind::Call { target, arguments } => ResolvedExpressionKind::Call {
                        target: match target {
                            CheckedCallTarget::Builtin(id) => {
                                ResolvedCallTarget::Builtin(facts.builtin(*id).to_owned())
                            }
                            CheckedCallTarget::Extern(reference) => {
                                ResolvedCallTarget::Extern(reference.id)
                            }
                            CheckedCallTarget::EnumVariant(variant) => {
                                let declaration = declarations.enum_variant_decl(*variant);
                                let owner = declarations.enum_decl(variant.owner);
                                ResolvedCallTarget::EnumVariant {
                                    enum_rust_name: owner.rust_name.clone(),
                                    variant_name: declaration.name.clone(),
                                }
                            }
                        },
                        arguments: arguments
                            .iter()
                            .map(|argument| match argument {
                                CheckedCallArgument::Value(value) => {
                                    ResolvedCallArgument::Value(*value)
                                }
                                CheckedCallArgument::Binding(binding) => {
                                    ResolvedCallArgument::Binding(*binding)
                                }
                            })
                            .collect(),
                    },
                    CheckedExprKind::Unary { operator, value } => ResolvedExpressionKind::Unary {
                        operator: match operator {
                            CheckedUnaryOperator::BooleanNot => ResolvedUnaryOperator::BooleanNot,
                            CheckedUnaryOperator::NumericNegation(_) => {
                                ResolvedUnaryOperator::NumericNegation
                            }
                        },
                        value: *value,
                    },
                    CheckedExprKind::Binary {
                        operator,
                        left,
                        right,
                    } => ResolvedExpressionKind::Binary {
                        operator: match operator {
                            CheckedBinaryOperator::Boolean(operator)
                            | CheckedBinaryOperator::Equality { op: operator, .. }
                            | CheckedBinaryOperator::Ordering { op: operator, .. }
                            | CheckedBinaryOperator::Arithmetic { op: operator, .. } => *operator,
                        },
                        left: *left,
                        right: *right,
                    },
                },
            })
            .collect();
        Self {
            values,
            locals,
            uses,
            nodes,
            daemon_window_local: facts.daemon_window_local(),
        }
    }

    pub(crate) fn value(&self, reference: ResolvedValueRef) -> &ResolvedValue {
        &self.values[&reference]
    }

    pub(crate) fn local(&self, id: ResolvedLocalId) -> &ResolvedLocal {
        &self.locals[id.0 as usize]
    }

    pub(crate) fn expression_use(&self, id: ResolvedExpressionId) -> &ResolvedExpressionUse {
        &self.uses[id.0 as usize]
    }

    pub(crate) fn expression(&self, id: ResolvedExpressionNodeId) -> &ResolvedExpressionNode {
        &self.nodes[id.0 as usize]
    }

    pub(crate) fn try_expression(
        &self,
        id: ResolvedExpressionNodeId,
    ) -> Option<&ResolvedExpressionNode> {
        self.nodes.get(id.0 as usize)
    }

    pub(crate) fn daemon_window_local(&self) -> Option<ResolvedLocalId> {
        self.daemon_window_local
    }
}
