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

/// A nonce no other fixture in this process shares. Fixture directories used
/// to be named from `SystemTime::now().as_nanos()`, which two threads
/// starting together can read identically — both then built the same path and
/// the first to finish deleted the directory out from under the second,
/// failing an unrelated test roughly one full run in three.
pub(crate) fn unique_nonce() -> u64 {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}
