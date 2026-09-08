//! Owned data expressions shared by surface routes and state snapshots.
use super::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ValueTarget {
    Surface,
    Snapshot,
}
impl ValueTarget {
    fn path(self) -> &'static str {
        match self {
            Self::Surface => "::ui_lang_guest::wire::SurfaceValue",
            Self::Snapshot => "::ui_lang_guest::wire::SnapshotValue",
        }
    }
}
fn refused(what: &str) -> String {
    what.into()
}

/// Encode a borrowed expression or decode an owned wire value. The generated
/// decoder returns Option<T>, so one mismatched nested field drops the event.
pub(super) fn code(
    ty: &Type,
    value: &str,
    decode: bool,
    program: &LoweredProgram,
    target: ValueTarget,
    visiting: &mut Vec<String>,
) -> Result<String, String> {
    let v = target.path();
    let scalar = match ty {
        Type::Unit => Some("Unit"),
        Type::Bool => Some("Bool"),
        Type::I64 => Some("I64"),
        Type::F64 => Some("F64"),
        Type::Str => Some("Str"),
        Type::Editor if target == ValueTarget::Snapshot => Some("Str"),
        Type::Bytes if target == ValueTarget::Snapshot => Some("Bytes"),
        _ => None,
    };
    if let Some(tag) = scalar {
        return Ok(if decode {
            let pattern = if tag == "Unit" {
                format!("{v}::Unit")
            } else {
                format!("{v}::{tag}(__item)")
            };
            let guard = if tag == "F64" {
                " if __item.is_finite()"
            } else {
                ""
            };
            let result = if tag == "Unit" { "()" } else { "__item" };
            format!(
                "match {value} {{ {pattern}{guard} => ::std::option::Option::Some({result}), _ => ::std::option::Option::None }}"
            )
        } else {
            match tag {
                "Unit" => format!("{{ let _ = {value}; {v}::Unit }}"),
                "Str" => format!("{v}::Str(::std::string::ToString::to_string({value}))"),
                "Bytes" => format!("{v}::Bytes(({value}).clone())"),
                _ => format!("{v}::{tag}(*({value}))"),
            }
        });
    }
    if target == ValueTarget::Snapshot {
        match ty {
            Type::Combo(inner) => {
                let list = Type::List(inner.clone());
                return Ok(if decode {
                    let options = code(&list, "__options", true, program, target, visiting)?;
                    format!(
                        "(|| {{ let {v}::Record {{ name, fields }} = {value} else {{ return None; }}; if name != \"combo\" || fields.len() != 2 {{ return None; }} let mut fields = fields.into_iter(); let (name, __options) = fields.next()?; if name != \"options\" {{ return None; }} let options = ({options})?; let (name, reset) = fields.next()?; if name != \"reset\" {{ return None; }} let {v}::Bytes(reset) = reset else {{ return None; }}; let reset = u64::from_le_bytes(reset.try_into().ok()?); Some(::ui_lang_guest::Combo::restore(options, reset)) }})()"
                    )
                } else {
                    let options = code(
                        &list,
                        &format!("({value}).options()"),
                        false,
                        program,
                        target,
                        visiting,
                    )?;
                    format!(
                        "{v}::Record {{ name: \"combo\".into(), fields: vec![(\"options\".into(), {options}), (\"reset\".into(), {v}::Bytes(({value}).reset_revision().to_le_bytes().to_vec()))] }}"
                    )
                });
            }
            Type::Markdown => {
                return Ok(if decode {
                    format!(
                        "match {value} {{ {v}::Str(__text) => ::std::option::Option::Some(::ui_lang_guest::Markdown::parse(&__text)), _ => ::std::option::Option::None }}"
                    )
                } else {
                    format!("{v}::Str(({value}).source().to_owned())")
                });
            }
            Type::KeyModifiers => {
                return Ok(if decode {
                    format!(
                        "match {value} {{ {v}::I64(__bits) => u32::try_from(__bits).ok().and_then(::iced::keyboard::Modifiers::from_bits), _ => ::std::option::Option::None }}"
                    )
                } else {
                    format!("{v}::I64(i64::from(({value}).bits()))")
                });
            }
            Type::Result(ok, error) => {
                return enum_code(
                    "result",
                    &[
                        (
                            "ok".into(),
                            "::std::result::Result::Ok".into(),
                            Some((**ok).clone()),
                        ),
                        (
                            "error".into(),
                            "::std::result::Result::Err".into(),
                            Some((**error).clone()),
                        ),
                    ],
                    value,
                    decode,
                    program,
                    visiting,
                );
            }
            Type::Palette(contract) => {
                let variants = program
                    .theme()
                    .palettes
                    .iter()
                    .map(|palette| {
                        (
                            palette.name.clone(),
                            format!(
                                "{}::{}",
                                canonical_rust_type_name(contract),
                                pascal(&palette.name)
                            ),
                            None,
                        )
                    })
                    .collect::<Vec<_>>();
                return enum_code(contract, &variants, value, decode, program, visiting);
            }
            Type::Named(name)
                if program
                    .enum_declarations()
                    .iter()
                    .any(|item| &item.name == name) =>
            {
                if visiting.contains(name) {
                    return Err("recursive snapshot state".into());
                }
                visiting.push(name.clone());
                let item = program
                    .enum_declarations()
                    .iter()
                    .find(|item| &item.name == name)
                    .unwrap();
                let variants = item
                    .variants
                    .iter()
                    .map(|variant| {
                        (
                            variant.name.clone(),
                            format!("{}::{}", item.rust_name, pascal(&variant.name)),
                            variant.payload.clone(),
                        )
                    })
                    .collect::<Vec<_>>();
                let result = enum_code(name, &variants, value, decode, program, visiting);
                visiting.pop();
                return result;
            }
            _ => {}
        }
    }
    match ty {
        Type::List(inner) => {
            let item = code(inner, "__item", decode, program, target, visiting)?;
            Ok(if decode {
                format!(
                    "match {value} {{ {v}::List(__items) => __items.into_iter().map(|__item| {item}).collect::<::std::option::Option<::std::vec::Vec<_>>>(), _ => ::std::option::Option::None }}"
                )
            } else {
                format!("{v}::List(({value}).iter().map(|__item| {item}).collect())")
            })
        }
        Type::Option(inner) => {
            let item = code(
                inner,
                if decode { "*__item" } else { "__item" },
                decode,
                program,
                target,
                visiting,
            )?;
            Ok(if decode {
                format!(
                    "match {value} {{ {v}::Option(::std::option::Option::None) => ::std::option::Option::Some(::std::option::Option::None), {v}::Option(::std::option::Option::Some(__item)) => ({item}).map(::std::option::Option::Some), _ => ::std::option::Option::None }}"
                )
            } else {
                format!(
                    "{v}::Option(({value}).as_ref().map(|__item| ::std::boxed::Box::new({item})))"
                )
            })
        }
        Type::Named(name) => {
            let declaration = program
                .struct_declarations()
                .iter()
                .find(|item| &item.name == name)
                .filter(|item| !item.fields.is_empty())
                .ok_or_else(|| refused("an opaque extern widget value"))?;
            if visiting.contains(name) {
                return Err(refused("a recursive extern widget value"));
            }
            visiting.push(name.clone());
            let mut fields = Vec::new();
            for (index, field) in declaration.fields.iter().enumerate() {
                let field_value = if decode {
                    format!("__field_{index}")
                } else {
                    format!("&({value}).{}", field.name)
                };
                let item = code(&field.ty, &field_value, decode, program, target, visiting)?;
                fields.push(if decode {
                    format!("{}: ({item})?", field.name)
                } else {
                    format!("(::std::string::String::from({:?}), {item})", field.name)
                });
            }
            visiting.pop();
            if decode {
                let extract = declaration.fields.iter().enumerate().map(|(index, field)| format!(
                    "let (__name, __field_{index}) = __fields.next()?; if __name != {:?} {{ return ::std::option::Option::None; }}", field.name
                )).collect::<Vec<_>>().join(" ");
                Ok(format!(
                    "(|| {{ let {v}::Record {{ name: __name, fields: __fields }} = {value} else {{ return ::std::option::Option::None; }}; if __name != {name:?} || __fields.len() != {} {{ return ::std::option::Option::None; }} let mut __fields = __fields.into_iter(); {extract} ::std::option::Option::Some({} {{ {} }}) }})()",
                    declaration.fields.len(),
                    declaration.rust_path,
                    fields.join(", ")
                ))
            } else {
                Ok(format!(
                    "{v}::Record {{ name: ::std::string::String::from({name:?}), fields: ::std::vec![{}] }}",
                    fields.join(", ")
                ))
            }
        }
        _ => Err(refused("a non-data extern widget value")),
    }
}

fn enum_code(
    name: &str,
    variants: &[(String, String, Option<Type>)],
    value: &str,
    decode: bool,
    program: &LoweredProgram,
    visiting: &mut Vec<String>,
) -> Result<String, String> {
    let v = ValueTarget::Snapshot.path();
    let mut arms = Vec::new();
    for (variant, path, payload) in variants {
        let item = match payload {
            Some(ty) => code(
                ty,
                if decode { "__payload" } else { "__item" },
                decode,
                program,
                ValueTarget::Snapshot,
                visiting,
            )?,
            None => format!("{v}::Unit"),
        };
        arms.push(if decode {
            let result = if payload.is_some() {
                format!("({item}).map({path})")
            } else {
                format!("matches!(__payload, {v}::Unit).then_some({path})")
            };
            format!("{variant:?} => {result}")
        } else {
            let pattern = if payload.is_some() { format!("{path}(__item)") } else { path.clone() };
            format!("{pattern} => {v}::Record {{ name: ::std::string::String::from({name:?}), fields: vec![(::std::string::String::from({variant:?}), {item})] }}")
        });
    }
    Ok(if decode {
        format!(
            "(|| {{ let {v}::Record {{ name: __name, fields: __fields }} = {value} else {{ return ::std::option::Option::None; }}; if __name != {name:?} || __fields.len() != 1 {{ return ::std::option::Option::None; }} let (__variant, __payload) = __fields.into_iter().next()?; match __variant.as_str() {{ {}, _ => ::std::option::Option::None }} }})()",
            arms.join(", ")
        )
    } else {
        format!("match {value} {{ {} }}", arms.join(", "))
    })
}
