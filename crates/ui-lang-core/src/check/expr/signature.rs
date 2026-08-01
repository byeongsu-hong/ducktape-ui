use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ContextualBuiltin {
    AnimationInterpolate,
    AnimationProject,
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
            "animation.interpolate" => Some(Self::AnimationInterpolate),
            "animation.project" => Some(Self::AnimationProject),
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
