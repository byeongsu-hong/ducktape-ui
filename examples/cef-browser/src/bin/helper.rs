fn main() {
    let exit_code = cef_browser_example::cef_runtime::run_helper();
    std::process::exit(exit_code.max(0));
}
