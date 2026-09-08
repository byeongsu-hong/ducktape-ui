#[cfg(not(target_arch = "wasm32"))]
fn main() -> Result<(), String> {
    app_store_editor_fixture::run_native()
}
#[cfg(target_arch = "wasm32")]
fn main() {}
