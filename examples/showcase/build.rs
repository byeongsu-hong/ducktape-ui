fn main() {
    ui_lang_build::compile_dir("src/ui").expect("compile showcase Ice sources");
    ui_lang_build::compile_dir("tests/cases/ui").expect("compile showcase Ice test fixtures");
}
