fn main() {
    ui_lang_build::compile_dir("src/ui").expect("compile app-store host Ice sources");
    let tests = ui_lang_build::compile_tree_tests("../apps/counter/src/ui/app.ice")
        .expect("compile Counter authored host tests");
    println!("cargo:rustc-env=COUNTER_TREE_TESTS={}", tests.display());
}
