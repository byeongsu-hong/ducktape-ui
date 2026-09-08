//! The same authored Ice scenario drives mounted widgets on each real backend.
use super::*;
use ui_lang_runtime::testing::{Config, Driver};

fn view(surface: &Surface) -> iced::Element<'_, String> {
    wasm_view(surface.clone(), false)
}

fn driver(
    native: bool,
    config: Config,
) -> Driver<impl iced::Program<State = Surface, Message = String, Theme = iced::Theme>> {
    let catalog = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(if native {
        "../target/app-store-native-catalog"
    } else {
        "../target/app-store-catalog"
    });
    let entry = crate::catalog::scan_dir(&catalog)
        .into_iter()
        .find(|entry| entry.name == "Counter")
        .expect("build the current Counter guest package before running authored tests");
    let surface = Surface(Arc::new(Mutex::new(Guest::load(&entry).unwrap())));
    let program = iced::application(
        move || (surface.clone(), iced::Task::none()),
        |surface: &mut Surface, _message: String| {
            assert!(
                surface.0.lock().unwrap().fault.is_none(),
                "guest must remain live"
            );
            iced::Task::none()
        },
        view,
    );
    Driver::new(program, config)
}

mod native {
    use super::*;
    fn __ice_tree_test_driver(
        config: Config,
    ) -> Driver<impl iced::Program<State = Surface, Message = String, Theme = iced::Theme>> {
        driver(true, config)
    }
    include!(env!("COUNTER_TREE_TESTS"));
}

mod wasm {
    use super::*;
    fn __ice_tree_test_driver(
        config: Config,
    ) -> Driver<impl iced::Program<State = Surface, Message = String, Theme = iced::Theme>> {
        driver(false, config)
    }
    include!(env!("COUNTER_TREE_TESTS"));
}
