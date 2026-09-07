//! Generated guest fixture for native and real wasm surface boundary tests.
ui_lang::include_app!("src/ui/app.ice");
ui_lang_guest::export_app!(
    SurfaceFixture,
    "Surface fixture",
    "Typed surface contract.",
    []
);
