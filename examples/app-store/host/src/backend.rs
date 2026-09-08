use super::{HostState, Store, View, arm, first_line, panic_message};
use crate::limits::FUEL_PER_TICK;
use ui_lang_wire::native::Request;

pub(super) enum Backend {
    Wasm {
        store: Store<HostState>,
        view: View,
    },
    Native(crate::native::Process),
    #[cfg(test)]
    Authored(Box<super::authored_backend::Backend>),
}

impl Backend {
    pub(super) fn init(&mut self, macos: bool) -> Result<(), String> {
        match self {
            #[cfg(test)]
            Self::Authored(_) => Err("begin an authored test explicitly".into()),
            Self::Wasm { store, view } => {
                arm(store);
                view.call_init(&mut *store, macos)
                    .map_err(|error| panic_message(store).unwrap_or_else(|| first_line(&error)))
            }
            Self::Native(process) => process.call(Request::Init { macos }).map(|_| ()),
        }
    }
    pub(super) fn tick(&mut self, bytes: &[u8]) -> Result<Vec<u8>, String> {
        match self {
            #[cfg(test)]
            Self::Authored(backend) => backend.call(ui_lang_wire::authored::Request::View(
                Request::Tick(bytes.to_vec()),
            )),
            Self::Wasm { store, view } => {
                arm(store);
                view.call_tick(&mut *store, bytes)
                    .map_err(|error| panic_message(store).unwrap_or_else(|| first_line(&error)))
            }
            Self::Native(process) => process.call(Request::Tick(bytes.to_vec())),
        }
    }
    pub(super) fn snapshot(&mut self) -> Result<Vec<u8>, String> {
        match self {
            #[cfg(test)]
            Self::Authored(backend) => {
                backend.call(ui_lang_wire::authored::Request::View(Request::Snapshot))
            }
            Self::Wasm { store, view } => {
                arm(store);
                view.call_snapshot(&mut *store)
                    .map_err(|error| panic_message(store).unwrap_or_else(|| first_line(&error)))?
            }
            Self::Native(process) => process.call(Request::Snapshot),
        }
    }
    pub(super) fn restore(&mut self, state: &[u8], macos: bool) -> Result<(), String> {
        match self {
            #[cfg(test)]
            Self::Authored(backend) => backend
                .call(ui_lang_wire::authored::Request::View(Request::Restore {
                    state: state.to_vec(),
                    macos,
                }))
                .map(|_| ()),
            Self::Wasm { store, view } => {
                arm(store);
                view.call_restore(&mut *store, state, macos)
                    .map_err(|error| panic_message(store).unwrap_or_else(|| first_line(&error)))?
            }
            Self::Native(process) => process
                .call(Request::Restore {
                    state: state.to_vec(),
                    macos,
                })
                .map(|_| ()),
        }
    }
    pub(super) fn fuel_used(&self) -> u64 {
        match self {
            Self::Wasm { store, .. } => FUEL_PER_TICK.saturating_sub(store.get_fuel().unwrap_or(0)),
            Self::Native(_) => 0,
            #[cfg(test)]
            Self::Authored(backend) => backend.fuel_used(),
        }
    }
}
