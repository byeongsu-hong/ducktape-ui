#[cfg(not(target_arch = "wasm32"))]
fn main() -> Result<(), String> {
    app_store_authored_input::run_native()
}

#[cfg(target_arch = "wasm32")]
fn main() {}
