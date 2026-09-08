fn main() {
    ui_lang_build::compile_dir_for("../../apps/counter/src/ui", ui_lang_build::Target::Tree)
        .expect("compile original Counter source graph");
    let tests = ui_lang_build::compile_tree_guest_tests("../../apps/counter/src/ui/app.ice")
        .expect("compile typed Counter test hooks");
    println!(
        "cargo:rustc-env=COUNTER_TREE_GUEST_TESTS={}",
        tests.display()
    );
}
