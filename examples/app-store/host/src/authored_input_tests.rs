//! Authored actions use native mounted input state, with guest-owned assertions.
use super::*;
use ui_lang_runtime::testing::{Config, Driver};

mod native {
    use super::*;
    fn __ice_tree_test_driver(
        config: Config,
        test: u32,
        fingerprint: u64,
    ) -> Driver<impl iced::Program<State = Surface, Message = String, Theme = iced::Theme>> {
        super::super::authored_tests::driver_for(
            super::super::authored_tests::fixture_entry(true, "authored-input"),
            config,
            test,
            fingerprint,
        )
    }
    use super::super::authored_tests::{__ice_tree_test_step, __ice_tree_test_target};
    include!(env!("INPUT_TREE_TESTS"));
}

mod wasm {
    use super::*;
    fn __ice_tree_test_driver(
        config: Config,
        test: u32,
        fingerprint: u64,
    ) -> Driver<impl iced::Program<State = Surface, Message = String, Theme = iced::Theme>> {
        super::super::authored_tests::driver_for(
            super::super::authored_tests::fixture_entry(false, "authored-input"),
            config,
            test,
            fingerprint,
        )
    }
    use super::super::authored_tests::{__ice_tree_test_step, __ice_tree_test_target};
    include!(env!("INPUT_TREE_TESTS"));
}
