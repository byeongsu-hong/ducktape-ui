use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ContextualBuiltin {
    Aborted,
    AnimationInterpolate,
    AnimationProject,
    DebugActive,
    DebugTimeWith,
    Empty,
    Len,
    LinearAddStops,
    MouseClick,
    Some,
    Ok,
    Err,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum BuiltinArgumentContext {
    Value {
        expected: Option<Type>,
    },
    Binding {
        ty: Type,
        body: usize,
    },
    ScopedValue {
        expected: Option<Type>,
        binding: usize,
    },
}

impl ContextualBuiltin {
    pub(crate) fn from_name(name: &str) -> Option<Self> {
        match name {
            "aborted" => Some(Self::Aborted),
            "animation.interpolate" => Some(Self::AnimationInterpolate),
            "animation.project" => Some(Self::AnimationProject),
            "debug.active" => Some(Self::DebugActive),
            "debug.time_with" => Some(Self::DebugTimeWith),
            "empty" => Some(Self::Empty),
            "len" => Some(Self::Len),
            "linear.add_stops" => Some(Self::LinearAddStops),
            "mouse.click" => Some(Self::MouseClick),
            "some" => Some(Self::Some),
            "ok" => Some(Self::Ok),
            "err" => Some(Self::Err),
            _ => None,
        }
    }

    pub(crate) fn argument_contexts(
        self,
        output: &Type,
        inferred_arguments: &[Type],
    ) -> Result<Vec<BuiltinArgumentContext>, &'static str> {
        let value = |expected| BuiltinArgumentContext::Value { expected };
        Ok(match self {
            Self::Aborted => vec![value(Some(Type::Option(Box::new(Type::TaskHandle))))],
            Self::DebugActive => vec![value(Some(Type::Option(Box::new(Type::DebugSpan))))],
            Self::DebugTimeWith => vec![value(Some(Type::Str)), value(Some(output.clone()))],
            Self::Empty | Self::Len => {
                let Some(input) = inferred_arguments.first() else {
                    return Err("collection query has no input");
                };
                let input = match input {
                    Type::List(inner) => Type::List(Box::new(resolve_erased_type(inner))),
                    Type::Str => Type::Str,
                    Type::Bytes => Type::Bytes,
                    Type::Secret => Type::Secret,
                    _ => {
                        return Err(
                            "collection query input is not a list, string, bytes, or secret",
                        );
                    }
                };
                vec![value(Some(input))]
            }
            Self::LinearAddStops => vec![
                value(Some(Type::LinearGradient)),
                value(Some(Type::List(Box::new(Type::ColorStop)))),
            ],
            Self::Some => match output {
                Type::Option(inner) => vec![value(Some(inner.as_ref().clone()))],
                _ => return Err("some output is not optional"),
            },
            Self::Ok => match output {
                Type::Result(ok, _) => vec![value(Some(ok.as_ref().clone()))],
                _ => return Err("ok output is not a result"),
            },
            Self::Err => match output {
                Type::Result(_, error) => vec![value(Some(error.as_ref().clone()))],
                _ => return Err("err output is not a result"),
            },
            Self::MouseClick => vec![
                value(Some(Type::Point)),
                value(Some(Type::MouseButton)),
                value(Some(Type::Option(Box::new(Type::MouseClick)))),
            ],
            Self::AnimationInterpolate => {
                let mut contexts = vec![
                    value(Some(Type::Animation(Box::new(Type::Bool)))),
                    value(Some(output.clone())),
                    value(Some(output.clone())),
                ];
                if inferred_arguments.len() == 4 {
                    contexts.push(value(Some(Type::Instant)));
                }
                contexts
            }
            Self::AnimationProject => {
                let Some(Type::Animation(inner)) = inferred_arguments.first() else {
                    return Err("animation.project input is not an animation");
                };
                let inner = inner.as_ref().clone();
                let mut contexts = vec![
                    value(Some(Type::Animation(Box::new(inner.clone())))),
                    BuiltinArgumentContext::Binding { ty: inner, body: 2 },
                    BuiltinArgumentContext::ScopedValue {
                        expected: Some(output.clone()),
                        binding: 1,
                    },
                ];
                if inferred_arguments.len() == 4 {
                    contexts.push(value(Some(Type::Instant)));
                }
                contexts
            }
        })
    }
}

pub(crate) fn resolve_erased_type(ty: &Type) -> Type {
    match ty {
        Type::Unknown => Type::Unit,
        Type::List(inner) => Type::List(Box::new(resolve_erased_type(inner))),
        Type::Option(inner) => Type::Option(Box::new(resolve_erased_type(inner))),
        Type::Result(output, error) => Type::Result(
            Box::new(resolve_erased_type(output)),
            Box::new(resolve_erased_type(error)),
        ),
        Type::Combo(inner) => Type::Combo(Box::new(resolve_erased_type(inner))),
        Type::Animation(inner) => Type::Animation(Box::new(resolve_erased_type(inner))),
        ty => ty.clone(),
    }
}

pub(crate) fn unify_type_evidence(left: &Type, right: &Type) -> Type {
    match (left, right) {
        (Type::Unknown, right) => right.clone(),
        (left, Type::Unknown) => left.clone(),
        (Type::List(left), Type::List(right)) => {
            Type::List(Box::new(unify_type_evidence(left, right)))
        }
        (Type::Option(left), Type::Option(right)) => {
            Type::Option(Box::new(unify_type_evidence(left, right)))
        }
        (Type::Result(left_output, left_error), Type::Result(right_output, right_error)) => {
            Type::Result(
                Box::new(unify_type_evidence(left_output, right_output)),
                Box::new(unify_type_evidence(left_error, right_error)),
            )
        }
        (Type::Combo(left), Type::Combo(right)) => {
            Type::Combo(Box::new(unify_type_evidence(left, right)))
        }
        (Type::Animation(left), Type::Animation(right)) => {
            Type::Animation(Box::new(unify_type_evidence(left, right)))
        }
        (left, _) => left.clone(),
    }
}
