//! Explicit test artifact from the identical shipped Counter source graph.
#[path = "../../../apps/counter/src/host.rs"]
pub mod host;
ui_lang::include_app!("../../apps/counter/src/ui/app.ice");
include!(env!("COUNTER_TREE_GUEST_TESTS"));
ui_lang_guest::export_test_app!(
    Counter,
    "Counter",
    "Authored Counter tests",
    ["clock", "bus"]
);
