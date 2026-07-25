pub(crate) fn load_example(file: &str) -> String {
    crate::source::load_test_source(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples/iced-app/src/ui")
            .join(file),
    )
    .unwrap()
}

macro_rules! example {
    ($file:literal) => {{
        static SOURCE: std::sync::OnceLock<String> = std::sync::OnceLock::new();
        SOURCE
            .get_or_init(|| crate::test_support::load_example($file))
            .as_str()
    }};
}

pub(crate) use example;
