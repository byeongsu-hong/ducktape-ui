fn main() {
    ui_lang_build::compile_dir("src/ui").expect("compile wasm-view guest Ice sources");
}
