fn main() {
    #[cfg(target_os = "linux")]
    if std::env::var_os("CARGO_FEATURE_CEF").is_some() {
        println!("cargo:rustc-link-arg-bin=cef-browser-example=-Wl,-rpath,$ORIGIN");
    }
}
