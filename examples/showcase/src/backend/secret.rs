use super::*;

/// The one function allowed to look at a secret buffer.
///
/// It stands in for a key derivation: what matters for the fixture is the
/// shape of the seam, not the arithmetic. `Secret` arrives by value, is read
/// once through `expose`, and wipes itself when this function returns — there
/// is no way to keep it that does not begin with copying it deliberately.
pub async fn derive_address(phrase: ui_lang_runtime::Secret) -> Result<String, AppError> {
    let words = phrase.expose().split_whitespace().count();
    if words < 3 {
        return Err(AppError {
            message: "A recovery phrase is at least three words.".into(),
        });
    }
    Ok(format!("0x{words}"))
}
