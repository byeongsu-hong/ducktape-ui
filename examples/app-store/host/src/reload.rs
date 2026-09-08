//! Prepare on the executor; commit only after the UI revalidates its request.
use super::*;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct InstallRequest {
    pub entry: CatalogEntry,
    pub serial: i64,
}
pub fn install_request(entry: CatalogEntry, serial: i64) -> InstallRequest {
    InstallRequest { entry, serial }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct InstallCompletion {
    pub serial: i64,
    result: Result<Loaded, StoreError>,
}

pub async fn install_requested(request: InstallRequest) -> InstallCompletion {
    InstallCompletion {
        serial: request.serial,
        result: install_app(request.entry).await,
    }
}

/// Startup restoration keeps independent apps, but never revives removed consent.
pub fn restore_current(
    library: &[Installed],
    opening: &[Loaded],
    running: &[Running],
    app: &Loaded,
) -> bool {
    library
        .iter()
        .any(|pin| pin.id == app.id && pin.hash == app.hash)
        && !opening.iter().any(|pending| pending.id == app.id)
        && !running.iter().any(|running| running.id == app.id)
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct InstallCommit {
    pub library: Vec<Installed>,
    pub opening: Vec<Loaded>,
    pub status: String,
    pub open: Option<Loaded>,
}

/// Revalidate the approval before persisting a pin or requesting a window.
pub fn commit_install(
    library: Vec<Installed>,
    opening: Vec<Loaded>,
    running: &[Running],
    serial: i64,
    completion: InstallCompletion,
) -> InstallCommit {
    let mut committed = InstallCommit {
        library,
        opening,
        status: String::new(),
        open: None,
    };
    if completion.serial != serial {
        return committed;
    }
    match completion.result {
        Ok(app) => {
            if running.iter().any(|running| running.id == app.id)
                || committed.opening.iter().any(|opening| opening.id == app.id)
            {
                return committed;
            }
            committed.library = add_to_library(committed.library, app.id.clone(), app.hash.clone());
            committed.opening = enqueue(committed.opening, app.clone());
            committed.open = Some(app);
        }
        Err(error) => committed.status = error.message,
    }
    committed
}

#[derive(Clone)]
pub struct Reload(Arc<Mutex<Prepared>>);

struct Prepared {
    serial: i64,
    running: Option<Running>,
    candidate: Result<Option<Candidate>, String>,
}

struct Candidate {
    entry: CatalogEntry,
    instance: Instance,
    frame: wire::Frame,
    alive: Arc<AtomicBool>,
    ticks: u64,
}

impl std::fmt::Debug for Reload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Reload")
            .field(&Arc::as_ptr(&self.0))
            .finish()
    }
}
impl PartialEq for Reload {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for Reload {}
impl Hash for Reload {
    fn hash<H: Hasher>(&self, state: &mut H) {
        Arc::as_ptr(&self.0).hash(state);
    }
}

pub async fn prepare_reload(entry: CatalogEntry, running: Vec<Running>, serial: i64) -> Reload {
    let running = running.into_iter().find(|app| app.id == entry.id);
    let candidate = match &running {
        Some(app) => Candidate::prepare(entry, &app.surface).map(Some),
        None => Err("The app is no longer running".into()),
    };
    Reload(Arc::new(Mutex::new(Prepared {
        serial,
        running,
        candidate,
    })))
}

pub fn reload_current(reload: &Reload, serial: i64) -> bool {
    reload.0.lock().expect("reload lock").serial == serial
}

pub fn finish_reload(
    running: &[Running],
    serial: i64,
    reload: Reload,
) -> Result<Loaded, StoreError> {
    finish(running, serial, reload).map_err(|message| StoreError { message })
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ReloadCommit {
    pub library: Vec<Installed>,
    pub running: Vec<Running>,
    pub status: String,
}

/// Called synchronously by the UI: validity, swap and pin share one update.
pub fn commit_reload(
    library: Vec<Installed>,
    running: Vec<Running>,
    serial: i64,
    reload: Reload,
) -> ReloadCommit {
    match finish_reload(&running, serial, reload) {
        Ok(loaded) => ReloadCommit {
            running: renamed_running(running, &loaded),
            library: add_to_library(library, loaded.id, loaded.hash),
            status: String::new(),
        },
        Err(error) => ReloadCommit {
            library,
            running,
            status: error.message,
        },
    }
}

fn finish(running: &[Running], serial: i64, reload: Reload) -> Result<Loaded, String> {
    let mut prepared = reload.0.lock().expect("reload lock");
    if prepared.serial != serial {
        return Err("The replacement request was superseded".into());
    }
    let app = prepared
        .running
        .clone()
        .ok_or("The app is no longer running")?;
    if !running.iter().any(|current| current == &app) {
        return Err("The app window changed while preparing its replacement".into());
    }
    let candidate = prepared
        .candidate
        .as_mut()
        .map_err(|error| error.clone())?
        .take()
        .ok_or("This replacement was already consumed")?;
    let mut guest = app.surface.0.lock().expect("guest lock");
    if !Arc::ptr_eq(&candidate.alive, &guest.alive) || candidate.ticks != guest.ticks {
        return Err(
            "The app changed while preparing its replacement; retry with its current state".into(),
        );
    }
    ensure_settled(&guest)?;
    let terminal = if candidate
        .entry
        .capabilities
        .iter()
        .any(|cap| cap.name == "terminal")
    {
        match &guest.terminal {
            Some(terminal) => Some(terminal.clone()),
            None => Some(Arc::new(Mutex::new(crate::terminal::Terminal::configured(
                std::env::var_os("ICE_TERMINAL_PROGRAM").map(Into::into),
            )?))),
        }
    } else {
        None
    };
    let entry = candidate.entry;
    let mut fresh = Guest::from_instance(
        &entry,
        candidate.instance,
        terminal,
        guest.log_session.clone(),
    );
    for (name, provider) in &guest.surfaces {
        if name != "terminal" {
            fresh.surfaces.insert(name.clone(), provider.clone());
        }
    }
    fresh.frame = candidate.frame;
    fresh.frame_rev = guest.frame_rev + 1;
    fresh.dark = guest.dark;
    fresh.inputs = std::mem::take(&mut guest.inputs);
    fresh.pictures = std::mem::take(&mut guest.pictures);
    if let Some(root) = &mut fresh.frame.root {
        fresh.inputs.adopt_after_reload(root);
        fresh.pictures.adopt(root);
        root.for_each_mut(&mut |node| match node {
            wire::Node::Svg { bytes, .. } => *bytes = None,
            wire::Node::Image { data, .. } => *data = None,
            _ => {}
        });
    }
    // Keep the candidate's first frame requests/cancels for the first redraw,
    // where native clipboard and mounted widget operations are available.
    fresh.staged_frame = true;
    *guest = fresh;
    drop(guest);
    Ok(Loaded {
        preferred_size: entry.preferred_size,
        id: entry.id,
        name: entry.name,
        hash: entry.hash,
        surface: app.surface,
    })
}

fn ensure_settled(guest: &Guest) -> Result<(), String> {
    if guest.fault.is_some()
        || guest.staged_frame
        || !guest.pending.is_empty()
        || !guest.widgets.is_empty()
        || !guest.clipboard.is_empty()
        || !guest.frame.requests.is_empty()
    {
        return Err("The app has pending work; retry after it settles".into());
    }
    Ok(())
}

impl Candidate {
    fn prepare(entry: CatalogEntry, surface: &Surface) -> Result<Self, String> {
        // Compile first, so edits during a cold compile are included in capture.
        let mut instance = Instance::new(&entry)?;
        let (snapshot, alive, ticks) = {
            let mut guest = surface.0.lock().expect("guest lock");
            ensure_settled(&guest)?;
            let snapshot = guest.backend.snapshot()?;
            (snapshot, guest.alive.clone(), guest.ticks)
        };
        // Host bounds apply even if a hostile guest ignores the SDK codec.
        wire::Snapshot::decode(&snapshot)?;
        instance
            .backend
            .restore(&snapshot, cfg!(target_os = "macos"))?;
        let bytes = instance
            .backend
            .tick(&wire::encode(&Vec::<wire::Event>::new()))?;
        let frame = shape(&bytes)?;
        if frame.root.is_none() {
            return Err("The replacement did not publish a complete tree".into());
        }
        Ok(Self {
            entry,
            instance,
            frame,
            alive,
            ticks,
        })
    }
}

#[cfg(test)]
mod install_tests {
    use super::*;

    #[test]
    #[ignore = "requires bundled reload fixture and isolated APP_STORE_DATA"]
    fn bundled_reload_install_completion_requires_current_request() {
        assert!(
            std::env::var_os("APP_STORE_DATA").is_some(),
            "set APP_STORE_DATA to an isolated test directory"
        );
        let data = crate::capabilities::storage::data_dir();
        std::fs::create_dir_all(&data).unwrap();
        let pins_path = data.join("installed");
        std::fs::write(&pins_path, "kept\told\n").unwrap();
        let original_pins = std::fs::read(&pins_path).unwrap();
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../target/reload-v1-fixture/app_store_reload_v1_fixture.wasm");
        let mut entry = crate::catalog::scan_dir(path.parent().unwrap())
            .into_iter()
            .find(|entry| std::path::Path::new(&entry.path) == path)
            .expect("bundle the current-format reload fixture");
        entry.id = "install-completion".into();
        let completion =
            iced::futures::executor::block_on(install_requested(install_request(entry, 7)));
        assert!(
            completion.result.is_ok(),
            "actual module must load before testing completion acceptance"
        );
        let app = completion.result.as_ref().unwrap();
        let approved = vec![Installed {
            id: app.id.clone(),
            hash: app.hash.clone(),
        }];
        assert!(
            restore_current(&approved, &[], &[], app),
            "an approved startup app still opens independently"
        );
        assert!(
            !restore_current(&[], &[], &[], app),
            "a startup completion cannot revive uninstalled consent"
        );
        assert!(
            !restore_current(
                &[Installed {
                    id: app.id.clone(),
                    hash: "new approval".into()
                }],
                &[],
                &[],
                app
            ),
            "a startup completion cannot replace newer consent"
        );
        assert!(
            !restore_current(&approved, std::slice::from_ref(app), &[], app),
            "startup and a manual load cannot open duplicate windows"
        );
        let pins = vec![Installed {
            id: "kept".into(),
            hash: "old".into(),
        }];
        let stale = commit_install(pins.clone(), vec![], &[], 8, completion.clone());
        assert_eq!(
            stale.library, pins,
            "a superseded completion cannot restore a pin"
        );
        assert!(
            stale.opening.is_empty(),
            "a superseded completion cannot enqueue a window"
        );
        assert!(
            stale.open.is_none(),
            "a superseded completion cannot request a native window"
        );
        assert_eq!(
            std::fs::read(&pins_path).unwrap(),
            original_pins,
            "a superseded completion cannot persist a pin"
        );

        let placements = crate::library::prepare_window(vec![], &stale.open);
        assert!(
            iced_test::runtime::task::into_stream(crate::library::open_guest(
                stale.open, placements
            ))
            .is_none(),
            "stale completion emits no native window action"
        );

        let current = commit_install(pins, vec![], &[], 7, completion);
        assert!(
            current.open.is_some(),
            "the current approval still opens its app"
        );
        use iced::futures::StreamExt;
        use iced_test::runtime::{Action, task, window};
        let placements = crate::library::prepare_window(vec![], &current.open);
        let mut native =
            task::into_stream(crate::library::open_guest(current.open, placements)).unwrap();
        let Some(Action::Window(window::Action::Open(_, settings, _))) =
            iced::futures::executor::block_on(native.next())
        else {
            panic!("current install opens the native window")
        };
        assert_eq!(settings.size, iced::Size::new(600.5, 400.25));
        assert_eq!(current.opening.len(), 1);
        assert_eq!(current.opening[0].id, "install-completion");
        assert!(
            current
                .library
                .iter()
                .any(|pin| pin.id == "install-completion")
        );
        assert!(
            std::fs::read_to_string(pins_path)
                .unwrap()
                .contains("install-completion\t")
        );
    }
}
