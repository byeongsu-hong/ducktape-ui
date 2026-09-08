//! The same authored Ice scenario drives mounted widgets on each real backend.
use super::*;
use ui_lang_runtime::testing::{Config, Driver};

fn view(surface: &Surface) -> iced::Element<'_, String> {
    wasm_view(surface.clone(), false)
}

fn driver(
    native: bool,
    config: Config,
    test: u32,
    fingerprint: u64,
) -> Driver<impl iced::Program<State = Surface, Message = String, Theme = iced::Theme>> {
    let entry = test_entry(native);
    let started = Instant::now();
    let mut backend = authored_backend::Backend::load(&entry).expect("load authored test artifact");
    let rejected = backend.call(wire::authored::Request::Begin {
        test,
        fingerprint: fingerprint ^ 1,
        macos: cfg!(target_os = "macos"),
    });
    assert!(
        rejected
            .unwrap_err()
            .contains("authored test artifact is stale")
    );
    backend
        .call(wire::authored::Request::Begin {
            test,
            fingerprint,
            macos: cfg!(target_os = "macos"),
        })
        .expect("begin exact authored test artifact");
    let instance = Instance {
        backend: Backend::Authored(Box::new(backend)),
        load: Load {
            took: started.elapsed(),
            cached: false,
        },
    };
    let guest = Guest::from_instance(
        &entry,
        instance,
        None,
        Arc::new(crate::surfaces::log::Session::default()),
    );
    let surface = Surface(Arc::new(Mutex::new(guest)));
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
        test: u32,
        fingerprint: u64,
    ) -> Driver<impl iced::Program<State = Surface, Message = String, Theme = iced::Theme>> {
        driver(true, config, test, fingerprint)
    }
    include!(env!("COUNTER_TREE_TESTS"));
}

mod wasm {
    use super::*;
    fn __ice_tree_test_driver(
        config: Config,
        test: u32,
        fingerprint: u64,
    ) -> Driver<impl iced::Program<State = Surface, Message = String, Theme = iced::Theme>> {
        driver(false, config, test, fingerprint)
    }
    include!(env!("COUNTER_TREE_TESTS"));
}

fn test_entry(native: bool) -> CatalogEntry {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../target");
    let path = if native {
        root.join("authored-native/authored-counter.native")
    } else {
        root.join("authored-wasm/app_store_authored_counter.wasm")
    };
    let (manifest, hash) = if native {
        let (text, bytes) =
            crate::catalog::native_package_with(&path, wire::authored::parse_manifest)
                .expect("build authored native fixture");
        let manifest = wire::authored::parse_manifest(std::str::from_utf8(&text).unwrap())
            .expect("require an explicit authored test manifest");
        (manifest, crate::catalog::native_hash(&text, &bytes))
    } else {
        let bytes = std::fs::read(&path).expect("bundle authored Wasm fixture");
        let manifest = wire::authored::read_manifest(&bytes)
            .expect("require an explicit authored test component");
        (manifest, sha256_hex(&bytes))
    };
    assert!(
        crate::catalog::scan_dir(path.parent().unwrap()).is_empty(),
        "test artifacts never enter the production catalog"
    );
    CatalogEntry {
        id: "authored-counter".into(),
        name: manifest.name,
        description: manifest.description,
        capabilities: manifest
            .capabilities
            .into_iter()
            .map(|name| crate::catalog::Capability { name })
            .collect(),
        preferred_size: manifest.preferred_size,
        path: path.to_string_lossy().into_owned(),
        mark: "C".into(),
        hash,
    }
}

fn __ice_tree_test_step<P>(
    driver: &mut Driver<P>,
    test: u32,
    step: u32,
    location: ui_lang_runtime::testing::Location,
) where
    P: iced::Program<State = Surface, Message = String, Theme = iced::Theme> + 'static,
    P::Renderer: 'static,
{
    driver.redraw(location);
    {
        let mut guest = driver.state().0.lock().unwrap();
        let Backend::Authored(backend) = &mut guest.backend else {
            panic!("explicit test artifact required");
        };
        let result = backend.call(wire::authored::Request::Step { test, step });
        assert!(result.is_ok(), "{location}: {}", result.unwrap_err());
        // A typed dispatch must become a mounted frame before the next UI oracle.
        guest.pending.push(wire::Event::Resync);
    }
    driver.redraw(location);
}
