//! Complete owned state snapshots for Tree apps. No initializer runs on restore.
use super::*;
use sha2::{Digest, Sha256};

const VALUE: &str = "::ui_lang_guest::wire::SnapshotValue";

struct Field {
    name: String,
    ty: String,
    encode: String,
    decode: String,
}

fn data_field(program: &LoweredProgram, name: &str, ty: &Type) -> Result<Field, String> {
    Ok(Field {
        name: name.into(),
        ty: rust_type_code(program, ty),
        encode: value::code(
            ty,
            &format!("&self.{name}"),
            false,
            program,
            value::ValueTarget::Snapshot,
            &mut Vec::new(),
        )?,
        decode: value::code(
            ty,
            "__value",
            true,
            program,
            value::ValueTarget::Snapshot,
            &mut Vec::new(),
        )?,
    })
}

fn record_encode(name: &str, fields: &[Field]) -> String {
    let values = fields
        .iter()
        .map(|f| format!("(::std::string::String::from({:?}), {})", f.name, f.encode))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "{VALUE}::Record {{ name: ::std::string::String::from({name:?}), fields: vec![{values}] }}"
    )
}

fn record_decode(name: &str, fields: &[Field], result: &str) -> String {
    let extract = fields.iter().map(|f| format!(
        "let (__name, __value) = __fields.next()?; if __name != {:?} {{ return ::std::option::Option::None; }} let {}: {} = ({})?;", f.name, f.name, f.ty, f.decode
    )).collect::<Vec<_>>().join(" ");
    format!(
        "(|| {{ let {VALUE}::Record {{name: __name, fields: __fields}} = __value else {{ return ::std::option::Option::None; }}; if __name != {name:?} || __fields.len() != {} {{ return ::std::option::Option::None; }} let mut __fields = __fields.into_iter(); {extract} ::std::option::Option::Some({result}) }})()",
        fields.len()
    )
}

fn component_value(
    program: &LoweredProgram,
    component: &crate::lower::ComponentContract,
) -> Result<(String, String), String> {
    let fields = component
        .states
        .iter()
        .map(|s| data_field(program, &s.name, &s.ty))
        .collect::<Result<Vec<_>, _>>()?;
    // These expressions run with a borrowed component value, never an app.
    let encode = record_encode(&component.name, &fields).replace("&self.", "&__component.");
    let result = component_state_code(program, component, None);
    Ok((encode, record_decode(&component.name, &fields, &result)))
}

/// Materialize saved initial values with fresh memo revisions and task lanes.
pub(super) fn component_state_code(
    program: &LoweredProgram,
    component: &crate::lower::ComponentContract,
    initial: Option<&str>,
) -> String {
    let mut result = format!("{} {{", component_state_type(&component.name));
    for state in &component.states {
        let value = initial.map_or_else(
            || state.name.clone(),
            |initial| format!("{initial}.{}.clone()", state.name),
        );
        write!(result, "{}: {value},", state.name).unwrap();
    }
    result.push_str(&revisions_init_code(component.states.len()));
    for (lane, _, mode) in component_run_lanes(program, &component.handlers) {
        write!(result, "{}: 0,", run_lane_generation_field(lane.0 as usize)).unwrap();
        if mode == DeliveryMode::Replace {
            write!(
                result,
                "{}: ::std::option::Option::None,",
                run_lane_handle_field(lane.0 as usize)
            )
            .unwrap();
        }
    }
    result.push('}');
    result
}

fn component_fields(
    program: &LoweredProgram,
    component: &crate::lower::ComponentContract,
) -> Result<Vec<Field>, String> {
    let (encode, decode) = component_value(program, component)?;
    let field = component_state_field(&component.name);
    let ty = component_state_type(&component.name);
    let map_name = format!("{} instances", component.name);
    let values = if component.storage == ComponentStorage::Mounted {
        format!("self.{field}.values()")
    } else {
        format!("&self.{field}")
    };
    let map_encode = format!(
        "{{ let __values = {values}; let mut __scopes = __values.keys().collect::<::std::vec::Vec<_>>(); __scopes.sort(); {VALUE}::Record {{ name: ::std::string::String::from({map_name:?}), fields: __scopes.into_iter().map(|__scope| {{ let __component = &__values[__scope]; (__scope.clone(), {encode}) }}).collect() }} }}"
    );
    let map_decode = format!(
        "(|| {{ let {VALUE}::Record {{name: __name, fields: __fields}} = __value else {{ return ::std::option::Option::None; }}; if __name != {map_name:?} {{ return ::std::option::Option::None; }} let mut __values = ::std::collections::HashMap::new(); for (__scope, __value) in __fields {{ let __component = ({decode})?; if __values.insert(__scope, __component).is_some() {{ return ::std::option::Option::None; }} }} ::std::option::Option::Some(__values) }})()"
    );
    let map_ty = format!("::std::collections::HashMap<::std::string::String, {ty}>");
    let stored = if component.storage == ComponentStorage::Mounted {
        let mounted = [
            Field {
                name: "values".into(),
                ty: map_ty,
                encode: map_encode,
                decode: map_decode,
            },
            Field {
                name: "booted".into(),
                ty: "::std::collections::HashSet<::std::string::String>".into(),
                encode: format!(
                    "{{ let mut __scopes = self.{field}.booted_scopes(); __scopes.sort(); {VALUE}::List(__scopes.into_iter().map({VALUE}::Str).collect()) }}"
                ),
                decode: format!(
                    "(|| {{ let {VALUE}::List(__scopes) = __value else {{return ::std::option::Option::None;}}; let mut __booted = ::std::collections::HashSet::new(); for __scope in __scopes {{ let {VALUE}::Str(__scope) = __scope else {{return ::std::option::Option::None;}}; if !__booted.insert(__scope) {{return ::std::option::Option::None;}} }} ::std::option::Option::Some(__booted) }})()"
                ),
            },
        ];
        Field {
            name: field,
            ty: format!("::ui_lang_runtime::MountedComponentState<{ty}>"),
            encode: record_encode("mounted", &mounted),
            decode: record_decode(
                "mounted",
                &mounted,
                "::ui_lang_runtime::MountedComponentState::from_snapshot(values, booted)",
            ),
        }
    } else {
        Field {
            name: field,
            ty: map_ty,
            encode: map_encode,
            decode: map_decode,
        }
    };
    let initial = component_state_initial_field(&component.name);
    let mut fields = vec![
        stored,
        Field {
            name: initial.clone(),
            ty,
            encode: format!("{{ let __component = &self.{initial}; {encode} }}"),
            decode,
        },
    ];
    for state in component.states.iter().filter(|s| s.ty == Type::Editor) {
        fields.push(data_field(
            program,
            &component_editor_initial_field(&component.name, &state.name),
            &state.ty,
        )?);
    }
    Ok(fields)
}

fn schema_type(program: &LoweredProgram, ty: &Type, out: &mut String, seen: &mut Vec<String>) {
    write!(out, "{ty:?};").unwrap();
    match ty {
        Type::List(inner) | Type::Option(inner) => schema_type(program, inner, out, seen),
        Type::Result(ok, err) => {
            schema_type(program, ok, out, seen);
            schema_type(program, err, out, seen);
        }
        Type::Named(name) if !seen.contains(name) => {
            seen.push(name.clone());
            if let Some(item) = program
                .struct_declarations()
                .iter()
                .find(|s| &s.name == name)
            {
                for f in &item.fields {
                    write!(out, "{:?}:", f.name).unwrap();
                    schema_type(program, &f.ty, out, seen);
                }
            }
            if let Some(item) = program.enum_declarations().iter().find(|s| &s.name == name) {
                for v in &item.variants {
                    write!(out, "{:?}:", v.name).unwrap();
                    if let Some(ty) = &v.payload {
                        schema_type(program, ty, out, seen);
                    }
                }
            }
        }
        Type::Palette(_) => {
            for p in &program.theme().palettes {
                write!(out, "{:?};", p.name).unwrap();
            }
        }
        _ => {}
    }
}

fn state_fields(program: &LoweredProgram) -> Result<(Vec<Field>, String), String> {
    if !program.secrets().is_empty() {
        return Err("secret state cannot be snapshotted".into());
    }
    if document_pane_grids(program)
        .iter()
        .any(|(_, test_only)| !test_only)
    {
        return Err("native pane state cannot be snapshotted".into());
    }
    let mut fields = Vec::new();
    let mut schema = format!("snapshot-v1;{:?};", program.app_name());
    let mut seen = Vec::new();
    for state in program.app_states() {
        write!(schema, "root:{:?};", state.name).unwrap();
        schema_type(program, &state.ty, &mut schema, &mut seen);
        fields.push(
            data_field(program, &state.name, &state.ty)
                .map_err(|e| format!("state {}: {e}", state.name))?,
        );
    }
    for c in program
        .components()
        .iter()
        .filter(|c| c.storage != ComponentStorage::Stateless)
    {
        write!(schema, "component:{:?}:{:?};", c.name, c.storage).unwrap();
        for s in &c.states {
            write!(schema, "{:?};", s.name).unwrap();
            schema_type(program, &s.ty, &mut schema, &mut seen);
        }
        fields.extend(
            component_fields(program, c).map_err(|e| format!("component {}: {e}", c.name))?,
        );
    }
    Ok((
        fields,
        Sha256::digest(schema)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect(),
    ))
}

pub(super) fn generate(out: &mut String, program: &LoweredProgram) -> Result<(), Error> {
    let (fields, schema) = match state_fields(program) {
        Ok(state) => state,
        Err(reason) => {
            writeln!(out, "pub(crate) fn __snapshot(&self) -> ::std::result::Result<::std::vec::Vec<u8>, ::std::string::String> {{ ::std::result::Result::Err(::std::string::String::from({reason:?})) }}\npub(crate) fn __restore(_: &[u8]) -> ::std::result::Result<Self, ::std::string::String> {{ ::std::result::Result::Err(::std::string::String::from({reason:?})) }}").unwrap();
            return Ok(());
        }
    };
    let parameters = fields
        .iter()
        .map(|f| (f.name.clone(), f.ty.clone()))
        .collect::<Vec<_>>();
    application::generate_state_constructor(out, program, Some(&parameters))?;
    let encode = record_encode(program.app_name(), &fields);
    let arguments = fields
        .iter()
        .map(|f| f.name.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    let decode = record_decode(
        program.app_name(),
        &fields,
        &format!("Self::__restore_state({arguments})"),
    );
    writeln!(out, "pub(crate) const __SNAPSHOT_SCHEMA: &'static str = {schema:?};\npub(crate) fn __snapshot(&self) -> ::std::result::Result<::std::vec::Vec<u8>, ::std::string::String> {{ ::ui_lang_guest::wire::Snapshot {{schema: ::std::string::String::from(Self::__SNAPSHOT_SCHEMA), state: {encode}}}.encode() }}\npub(crate) fn __restore(__bytes: &[u8]) -> ::std::result::Result<Self, ::std::string::String> {{ let __snapshot = ::ui_lang_guest::wire::Snapshot::decode(__bytes)?; if __snapshot.schema != Self::__SNAPSHOT_SCHEMA {{ return ::std::result::Result::Err(::std::string::String::from(\"snapshot schema mismatch\")); }} let __value = __snapshot.state; ({decode}).ok_or_else(|| ::std::string::String::from(\"snapshot state mismatch\")) }}").unwrap();
    Ok(())
}
