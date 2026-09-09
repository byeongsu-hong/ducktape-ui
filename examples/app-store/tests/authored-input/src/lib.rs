//! Explicit authored test artifact: native widget input reaches guest state.
ui_lang::include_app!("src/ui/app.ice");
include!(env!("INPUT_TREE_GUEST_TESTS"));
ui_lang_guest::export_test_app!(InputFixture, "Input", "Authored input tests", []);
