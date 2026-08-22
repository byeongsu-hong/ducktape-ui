use proc_macro::TokenStream;
use std::path::PathBuf;
use std::str::FromStr;

#[proc_macro]
pub fn include_app(input: TokenStream) -> TokenStream {
    expand(input).unwrap_or_else(|message| {
        TokenStream::from_str(&format!("compile_error!({message:?});"))
            .expect("compile_error token stream")
    })
}

fn expand(input: TokenStream) -> Result<TokenStream, String> {
    let relative = parse_literal(&input.to_string())?;
    let out_dir = std::env::var_os("OUT_DIR").ok_or_else(|| {
        "ui-lang: OUT_DIR is unavailable; generate Ice sources from build.rs with ui_lang_build"
            .to_owned()
    })?;
    let generated = ui_lang_build::generated_path(PathBuf::from(out_dir), &relative)
        .map_err(|error| error.to_string())?;
    if !generated.is_file() {
        return Err(format!(
            "ui-lang: generated Rust is missing for {relative}; call ui_lang_build::compile_dir from build.rs"
        ));
    }
    let expansion = format!("include!({:?});", generated.display().to_string());
    TokenStream::from_str(&expansion).map_err(|error| {
        format!("ui-lang generated an invalid include for {relative}: {error}\n{expansion}",)
    })
}

fn parse_literal(input: &str) -> Result<String, String> {
    let input = input.trim();
    if input.len() < 2 || !input.starts_with('"') || !input.ends_with('"') {
        return Err("ui_lang::include_app! expects one manifest-relative string literal".into());
    }
    let value = &input[1..input.len() - 1];
    if value.contains('\\') {
        return Err("ui_lang::include_app! paths must use `/` and cannot contain escapes".into());
    }
    let bytes = value.as_bytes();
    if bytes.get(1) == Some(&b':') && bytes[0].is_ascii_alphabetic()
        || PathBuf::from(value).components().any(|component| {
            matches!(
                component,
                std::path::Component::Prefix(_) | std::path::Component::RootDir
            )
        })
    {
        return Err(
            "ui_lang::include_app! paths must be relative to the manifest directory".into(),
        );
    }
    Ok(value.into())
}

#[cfg(test)]
mod tests {
    use super::parse_literal;

    #[test]
    fn include_paths_are_manifest_relative() {
        assert_eq!(parse_literal(r#""ui/app.ice""#).unwrap(), "ui/app.ice");
        assert_eq!(parse_literal(r#""../app.ice""#).unwrap(), "../app.ice");
        for path in [
            r#""/tmp/app.ice""#,
            r#""C:/tmp/app.ice""#,
            r#""ui\\app.ice""#,
        ] {
            assert!(parse_literal(path).is_err(), "accepted {path}");
        }
    }
}
