//! The installed list: what the user has installed, what the host brings
//! back at boot, and the labels the sidebar reads off both.

use std::sync::atomic::{AtomicUsize, Ordering};

use crate::capabilities::storage;
use crate::catalog::CatalogEntry;
use crate::store::{Surface, install_app};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct InstalledApp {
    pub id: String,
    pub name: String,
    pub surface: Surface,
}

/// What one restore brought back, and what it could not: a module that no
/// longer loads must not take the apps beside it down with it.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Restored {
    pub apps: Vec<InstalledApp>,
    /// Empty when everything came back — the status line's "nothing to say".
    pub failed: String,
}

/// Reinstalls whatever was installed when the host last exited, in the order
/// the file lists. Sequential on purpose: every install is a cranelift run,
/// and three at once would stall the first window for as long as the slowest.
/// An id the catalog no longer has is skipped — the module was deleted, which
/// is not an error the user can do anything about — and one that fails to
/// load is reported without stopping the rest.
pub async fn restore_installed(catalog: Vec<CatalogEntry>) -> Restored {
    let mut apps = Vec::new();
    let mut failed = Vec::new();
    for entry in remembered(&catalog) {
        match install_app(entry).await {
            Ok(app) => apps.push(app),
            Err(error) => failed.push(error.message),
        }
    }
    Restored {
        apps,
        failed: failed.join("; "),
    }
}

/// The restore takes seconds and the Install buttons stay live through it, so
/// what came back is merged into what the user has, never written over it.
/// One id is one app: the instance the user just installed is the newer one,
/// and one the user uninstalled meanwhile is gone from the file and must not
/// come back with the restore.
pub fn merge_installed(
    restored: Vec<InstalledApp>,
    current: Vec<InstalledApp>,
) -> Vec<InstalledApp> {
    let remembered = remembered_ids();
    let mut apps = restored;
    apps.retain(|app| {
        remembered.contains(&app.id) && !current.iter().any(|installed| installed.id == app.id)
    });
    apps.extend(current);
    apps
}

pub fn add_installed(mut apps: Vec<InstalledApp>, app: InstalledApp) -> Vec<InstalledApp> {
    remember(|ids| {
        ids.retain(|id| *id != app.id);
        ids.push(app.id.clone());
    });
    apps.retain(|installed| installed.id != app.id);
    apps.push(app);
    apps
}

/// Dropping the last handle drops the wasmtime store — the instance, its
/// memory and its compiled code go with it. That is the whole uninstall.
pub fn remove_installed(mut apps: Vec<InstalledApp>, id: String) -> Vec<InstalledApp> {
    remember(|ids| ids.retain(|remembered| *remembered != id));
    apps.retain(|installed| installed.id != id);
    apps
}

/// The ids to bring back at boot, one per line.
const INSTALLED_FILE: &str = "installed";

/// Edits the file, never rewrites it from the installed list: the list is
/// still missing whatever [`restore_installed`] is compiling, and an install
/// made meanwhile would otherwise leave a one-line file behind. Through a temp
/// file and a rename, like every other write here, so a crash loses at most
/// the last change and never the list.
fn remember(edit: impl FnOnce(&mut Vec<String>)) {
    let mut ids = remembered_ids();
    edit(&mut ids);
    let dir = storage::data_dir();
    let _ = std::fs::create_dir_all(&dir);
    let _ = storage::write_atomic(&dir.join(INSTALLED_FILE), ids.join("\n").as_bytes());
}

fn remembered_ids() -> Vec<String> {
    std::fs::read_to_string(storage::data_dir().join(INSTALLED_FILE))
        .unwrap_or_default()
        .lines()
        .map(str::to_string)
        .collect()
}

fn remembered(catalog: &[CatalogEntry]) -> Vec<CatalogEntry> {
    remembered_ids()
        .iter()
        .filter_map(|id| catalog.iter().find(|entry| entry.id == *id))
        .cloned()
        .collect()
}

/// What the status line says while [`restore_installed`] runs; empty when
/// there is nothing to restore, which is the status line's "nothing to say".
pub fn restoring_label(catalog: Vec<CatalogEntry>) -> String {
    match remembered(&catalog).len() {
        0 => String::new(),
        1 => "Restoring 1 app…".to_string(),
        count => format!("Restoring {count} apps…"),
    }
}

pub fn is_installed(apps: &[InstalledApp], id: String) -> bool {
    apps.iter().any(|installed| installed.id == id)
}

pub fn none_installed(apps: &[InstalledApp]) -> bool {
    apps.is_empty()
}

pub fn installing_label(entry: CatalogEntry) -> String {
    format!("Installing {}…", entry.name)
}

pub(crate) static LIVE_INSTANCES: AtomicUsize = AtomicUsize::new(0);
/// How many of those the host had to end. They still hold a window (and its
/// Restart button) but no longer run, so they are not live.
pub(crate) static FAULTED: AtomicUsize = AtomicUsize::new(0);

/// Takes the installed list and the lifecycle generation so it is recomputed
/// exactly when either changes — a trap or a restart moves the counts without
/// installing anything; the count itself is the number of `Guest`s alive.
pub fn live_label(_apps: &[InstalledApp], _generation: i64) -> String {
    let ended = FAULTED.load(Ordering::Relaxed);
    let live = LIVE_INSTANCES.load(Ordering::Relaxed).saturating_sub(ended);
    match ended {
        0 => format!("live wasm instances: {live}"),
        ended => format!("live wasm instances: {live} ({ended} ended)"),
    }
}
