use proc_macro::TokenStream;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::atomic::{AtomicU64, Ordering};

static MATERIALIZATION_ID: AtomicU64 = AtomicU64::new(0);

#[proc_macro]
pub fn include_app(input: TokenStream) -> TokenStream {
    expand(input).unwrap_or_else(|message| {
        TokenStream::from_str(&format!("compile_error!({message:?});"))
            .expect("compile_error token stream")
    })
}

fn expand(input: TokenStream) -> Result<TokenStream, String> {
    let relative = parse_literal(&input.to_string())?;
    let manifest = std::env::var("CARGO_MANIFEST_DIR")
        .map_err(|_| "ui-lang: CARGO_MANIFEST_DIR is unavailable".to_owned())?;
    let path = PathBuf::from(manifest).join(relative);
    let display = path.display().to_string();
    let compiled = ui_lang_core::compile_file(&path).map_err(|error| error.render(&display))?;
    let generated = materialize_generated(&path, &compiled.rust)?;
    let expansion = format!("include!({:?});", generated.display().to_string());
    TokenStream::from_str(&expansion).map_err(|error| {
        format!(
            "ui-lang generated an invalid include for {}: {error}\n{}",
            path.display(),
            expansion,
        )
    })
}

fn materialize_generated(source: &std::path::Path, rust: &str) -> Result<PathBuf, String> {
    let mut hasher = DefaultHasher::new();
    source.hash(&mut hasher);
    rust.hash(&mut hasher);
    let directory = std::env::temp_dir().join("ui-lang-generated");
    std::fs::create_dir_all(&directory).map_err(|error| {
        format!(
            "ui-lang cannot create generated source directory {}: {error}",
            directory.display()
        )
    })?;
    let path = directory.join(format!("{:016x}.rs", hasher.finish()));
    let current = std::fs::read_to_string(&path).ok();
    if current.as_deref() != Some(rust) {
        let temporary = directory.join(format!(
            ".{}-{}-{}.tmp",
            path.file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or("generated"),
            std::process::id(),
            MATERIALIZATION_ID.fetch_add(1, Ordering::Relaxed),
        ));
        std::fs::write(&temporary, rust).map_err(|error| {
            format!(
                "ui-lang cannot write generated Rust for {} to {}: {error}",
                source.display(),
                temporary.display()
            )
        })?;
        if let Err(error) = std::fs::rename(&temporary, &path)
            && !path.is_file()
        {
            let _ = std::fs::remove_file(&temporary);
            return Err(format!(
                "ui-lang cannot publish generated Rust for {} to {}: {error}",
                source.display(),
                path.display()
            ));
        }
        let _ = std::fs::remove_file(temporary);
    }
    Ok(path)
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
    use super::{materialize_generated, parse_literal};

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

    #[test]
    fn materializes_stable_generated_sources() {
        let source = std::env::temp_dir().join("ui-lang-source-map-test.ice");
        let first = materialize_generated(&source, "fn generated() {}\n").unwrap();
        let second = materialize_generated(&source, "fn generated() {}\n").unwrap();
        assert_eq!(first, second);
        std::fs::write(&first, "corrupt").unwrap();
        let repaired = materialize_generated(&source, "fn generated() {}\n").unwrap();
        assert_eq!(first, repaired);
        assert_eq!(
            std::fs::read_to_string(&first).unwrap(),
            "fn generated() {}\n"
        );
    }
}
