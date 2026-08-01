use serde_json::Value;
use std::path::Path;

pub(super) const CAPTURE_SCHEMA_VERSION: u64 = 2;
pub(super) const REVIEW_SCHEMA_VERSION: u64 = 1;

pub(super) fn require_schema_version(
    path: &Path,
    document: &Value,
    expected: u64,
    artifact: &str,
) -> Result<(), String> {
    let value = document.get("schema_version").ok_or_else(|| {
        format!(
            "{} omits integer field `schema_version` for {artifact}",
            path.display()
        )
    })?;
    let version = value.as_u64().ok_or_else(|| {
        format!(
            "{} field `schema_version` for {artifact} must be an unsigned integer",
            path.display()
        )
    })?;
    if version != expected {
        return Err(format!(
            "{} uses unsupported {artifact} schema version {version}; expected {expected}",
            path.display()
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn schema_versions_are_strict_integers() {
        let path = Path::new("capture.json");
        assert!(
            require_schema_version(
                path,
                &json!({ "schema_version": CAPTURE_SCHEMA_VERSION }),
                CAPTURE_SCHEMA_VERSION,
                "capture manifest",
            )
            .is_ok()
        );
        for invalid in [json!({}), json!({ "schema_version": "2" })] {
            assert!(
                require_schema_version(path, &invalid, CAPTURE_SCHEMA_VERSION, "capture manifest",)
                    .is_err()
            );
        }
        assert!(
            require_schema_version(
                path,
                &json!({ "schema_version": CAPTURE_SCHEMA_VERSION + 1 }),
                CAPTURE_SCHEMA_VERSION,
                "capture manifest",
            )
            .unwrap_err()
            .contains("unsupported")
        );
    }
}
