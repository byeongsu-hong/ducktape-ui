#[cfg(not(target_arch = "wasm32"))]
fn main() -> Result<(), String> {
    app_store_todo::run_native()
}

#[cfg(target_arch = "wasm32")]
fn main() {}
