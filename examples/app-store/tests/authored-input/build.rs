fn main() {
    ui_lang_build::compile_dir_for("src/ui", ui_lang_build::Target::Tree)
        .expect("compile Input source graph");
    let tests = ui_lang_build::compile_tree_guest_tests("src/ui/app.ice")
        .expect("compile typed Input test hooks");
    println!("cargo:rustc-env=INPUT_TREE_GUEST_TESTS={}", tests.display());
}
