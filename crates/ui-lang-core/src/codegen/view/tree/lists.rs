use super::*;

pub(in crate::codegen::view) fn keyed_column(
    keyed: &ResolvedKeyedColumn,
    identity: Option<&ResolvedViewIdentity>,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
    scope: &str,
    mut body: String,
) -> Result<String, Error> {
    let key = key_code(identity, "keyed", keyed.origin, scope, env, program)?;
    let pixels = |value: Option<CheckedExprUseId>| -> Result<String, Error> {
        Ok(option_code(
            value
                .map(|value| {
                    resolved_expr_use_code(program, value, env, ValueMode::Owned)
                        .map(|value| format!("({value}) as f32"))
                })
                .transpose()?,
        ))
    };
    let padding = &keyed.padding;
    let padding = ResolvedContainerPadding {
        all: padding.all,
        x: padding.x,
        y: padding.y,
        top: padding.top,
        right: padding.right,
        bottom: padding.bottom,
        left: padding.left,
    };
    let padding = edges_code(&padding, [0; 4], program, env)?;
    let width = dimension_code(keyed.width.as_ref(), false, program, env, keyed.origin)?;
    let height = dimension_code(keyed.height.as_ref(), false, program, env, keyed.origin)?;
    let spacing = pixels(keyed.spacing)?;
    let max_width = pixels(keyed.max_width)?;
    let virtual_row = pixels(keyed.virtual_row)?;
    let align = option_code(keyed.align.map(|align| {
        format!(
            "{WIRE}::AlignX::{}",
            match align {
                FlexAlignment::Start => "Left",
                FlexAlignment::Center => "Center",
                FlexAlignment::End => "Right",
            }
        )
    }));
    write!(body, " let (keys, children) = __children.into_iter().map(|(key, child)| ({WIRE}::ListKey::from(key), child)).unzip(); {WIRE}::Node::KeyedColumn {{ key: {key}, keys: Some(keys), children, background: None, border: None, spacing: {spacing}, padding: {padding}, width: {width}, height: {height}, max_width: {max_width}, align: {align}, virtual_row: {virtual_row} }} }}").unwrap();
    Ok(body)
}
