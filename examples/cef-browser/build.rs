fn main() {
    ui_lang_build::compile_dir("src/ui").expect("compile CEF browser Ice sources");

    #[cfg(target_os = "linux")]
    if std::env::var_os("CARGO_FEATURE_CEF").is_some() {
        println!("cargo:rustc-link-arg-bin=cef-browser-example=-Wl,-rpath,$ORIGIN");
    }
}
