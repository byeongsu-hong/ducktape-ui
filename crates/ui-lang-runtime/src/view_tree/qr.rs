//! Encode copied data using the same owned QR widget as native Ice.
use super::*;
use iced::widget::qr_code::{Data, ErrorCorrection, Version};

pub(super) fn render(key: &str, code: &wire::Qr) -> IceElement<'static, Output> {
    let correction = code.correction.map(|value| match value {
        wire::QrCorrection::Low => ErrorCorrection::Low,
        wire::QrCorrection::Medium => ErrorCorrection::Medium,
        wire::QrCorrection::Quartile => ErrorCorrection::Quartile,
        wire::QrCorrection::High => ErrorCorrection::High,
    });
    let data = code
        .payload
        .as_ref()
        .and_then(|payload| match code.version {
            Some(version) => Data::with_version(
                payload,
                match version {
                    wire::QrVersion::Normal(value) => Version::Normal(value),
                    wire::QrVersion::Micro(value) => Version::Micro(value),
                },
                correction.unwrap_or(ErrorCorrection::Medium),
            )
            .ok(),
            None => match correction {
                Some(correction) => Data::with_error_correction(payload, correction).ok(),
                None => Data::new(payload).ok(),
            },
        });
    let mut qr = crate::qr_code(data);
    match code.size {
        Some(wire::QrSize::Cell(value)) => qr = qr.cell_size(value),
        Some(wire::QrSize::Total(value)) => qr = qr.total_size(value),
        None => {}
    }
    if code.cell.is_some() || code.background.is_some() {
        let cell = code.cell;
        let background = code.background;
        qr = qr.style(move |theme| {
            let default = iced::widget::qr_code::default(theme);
            iced::widget::qr_code::Style {
                cell: cell.map(color).unwrap_or(default.cell),
                background: background.map(color).unwrap_or(default.background),
            }
        });
    }
    accessible(qr, StableId::new(key), Role::Image)
        .logical_id_maybe(cfg!(test).then_some(key))
        .label("QR code")
        .into()
}
