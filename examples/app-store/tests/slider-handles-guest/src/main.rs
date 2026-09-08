#[cfg(not(target_arch = "wasm32"))]
fn main() -> Result<(), String> {
    app_store_slider_handles_fixture::run_native()
}
#[cfg(target_arch = "wasm32")]
fn main() {}
