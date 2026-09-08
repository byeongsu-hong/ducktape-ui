use super::*;

pub(super) fn render(
    qr: &ResolvedQrCode,
    identity: Option<&ResolvedViewIdentity>,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
    scope: &str,
) -> Result<String, Error> {
    let payload = resolved_expr_use_code(program, qr.payload, env, ValueMode::Borrowed)?;
    let key = key_code(identity, "qr", qr.origin, scope, env, program)?;
    let (version, correction) = match qr.encoding {
        ResolvedQrEncoding::Auto { correction } => (None, correction),
        ResolvedQrEncoding::Versioned {
            version,
            correction,
        } => (Some(version), Some(correction)),
    };
    let version = version.map(|version| match version {
        ResolvedQrVersion::Normal(value) => format!("{WIRE}::QrVersion::Normal({value})"),
        ResolvedQrVersion::Micro(value) => format!("{WIRE}::QrVersion::Micro({value})"),
    });
    let correction = correction.map(|correction| {
        format!(
            "{WIRE}::QrCorrection::{}",
            match correction {
                ResolvedQrCorrection::Low => "Low",
                ResolvedQrCorrection::Medium => "Medium",
                ResolvedQrCorrection::Quartile => "Quartile",
                ResolvedQrCorrection::High => "High",
            }
        )
    });
    let size = match qr.size {
        ResolvedQrSize::Default => None,
        ResolvedQrSize::Cell(value) | ResolvedQrSize::Total(value) => {
            let kind = if matches!(qr.size, ResolvedQrSize::Cell(_)) {
                "Cell"
            } else {
                "Total"
            };
            let value = resolved_expr_use_code(program, value, env, ValueMode::Owned)?;
            Some(format!("{WIRE}::QrSize::{kind}(({value}) as f32)"))
        }
    };
    Ok(format!(
        "{WIRE}::Node::Qr {{ key: {key}, code: {WIRE}::Qr {{ payload: ::std::option::Option::Some(::std::convert::AsRef::<[u8]>::as_ref(&({payload})).to_vec()), version: {}, correction: {}, size: {}, cell: {}, background: {} }} }}",
        option_code(version),
        option_code(correction),
        option_code(size),
        option_code(qr.cell.as_ref().map(rgba_code)),
        option_code(qr.background.as_ref().map(rgba_code)),
    ))
}
