#[cfg(not(target_arch = "wasm32"))]
fn main() -> Result<(), String> {
    app_store_gradient_fixture::run_native()
}
#[cfg(target_arch = "wasm32")]
fn main() {}
