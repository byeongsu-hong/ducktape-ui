//! Test-only transport; ordinary catalog/load/export paths never opt into it.
use super::*;
use ui_lang_wire::{authored::Request, native::Request as ViewRequest};
mod bindings {
    macro_rules! bind {
        ($wit:literal) => { wasmtime::component::bindgen!({ inline: $wit, world: "view" }); };
    }
    ui_lang_wire::with_test_view_wit!(bind);
}
impl bindings::ViewImports for HostState {
    fn panicked(&mut self, message: String) {
        self.panic = Some(message);
    }
}
pub(super) enum Backend {
    Wasm {
        store: Store<HostState>,
        view: bindings::View,
    },
    Native(crate::native::Process),
}
impl Backend {
    pub(super) fn load(entry: &CatalogEntry) -> Result<Self, String> {
        if crate::catalog::is_native(entry) {
            return crate::native::Process::new_authored(entry).map(Self::Native);
        }
        let (component, _) = super::component_with_manifest(entry, wire::authored::read_manifest)?;
        let mut store = new_wasm_store();
        let mut linker = Linker::new(engine());
        bindings::View::add_to_linker::<HostState, wasmtime::component::HasSelf<HostState>>(
            &mut linker,
            |state| state,
        )
        .map_err(|error| error.to_string())?;
        linker
            .define_unknown_imports_as_traps(&component)
            .map_err(|error| error.to_string())?;
        let view = bindings::View::instantiate(&mut store, &component, &linker)
            .map_err(|error| error.to_string())?;
        Ok(Self::Wasm { store, view })
    }
    pub(super) fn call(&mut self, request: Request) -> Result<Vec<u8>, String> {
        match self {
            Self::Native(process) => process.call_authored(request),
            Self::Wasm { store, view } => {
                arm(store);
                match request {
                    Request::View(ViewRequest::Tick(bytes)) => view
                        .call_tick(store, &bytes)
                        .map_err(|error| error.to_string()),
                    Request::View(ViewRequest::Snapshot) => view
                        .call_snapshot(store)
                        .map_err(|error| error.to_string())?,
                    Request::View(ViewRequest::Restore { state, macos }) => view
                        .call_restore(store, &state, macos)
                        .map_err(|error| error.to_string())?
                        .map(|_| Vec::new()),
                    Request::View(ViewRequest::Init { .. }) => {
                        Err("begin an authored test explicitly".into())
                    }
                    request => view
                        .call_authored(store, &wire::encode(&request))
                        .map_err(|error| error.to_string())?,
                }
            }
        }
    }
    pub(super) fn fuel_used(&self) -> u64 {
        match self {
            Self::Wasm { store, .. } => FUEL_PER_TICK.saturating_sub(store.get_fuel().unwrap_or(0)),
            Self::Native(_) => 0,
        }
    }
}
