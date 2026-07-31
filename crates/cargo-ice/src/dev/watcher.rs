use super::inputs::{CargoInputGraph, normalize_watch_path};
use crate::ignored_dir;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::cmp::Ordering;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, TryRecvError};
use std::time::{Duration, Instant};

const FULL_RESCAN_INTERVAL: Duration = Duration::from_secs(30);

pub(super) struct DevWatcher {
    watcher: RecommendedWatcher,
    events: Receiver<notify::Result<Event>>,
    configuration: WatchConfiguration,
    pending_rescan: bool,
    last_snapshot: Instant,
}

impl DevWatcher {
    pub(super) fn new(
        dependencies: &[PathBuf],
        asset_dependencies: &[PathBuf],
        cargo_inputs: &CargoInputGraph,
    ) -> Result<Self, String> {
        let configuration = WatchConfiguration::new(dependencies, asset_dependencies, cargo_inputs);
        let (watcher, events) =
            create_watcher(&configuration.roots, &configuration.excluded_roots)?;
        Ok(Self {
            watcher,
            events,
            configuration,
            pending_rescan: true,
            last_snapshot: Instant::now(),
        })
    }

    pub(super) fn update(
        &mut self,
        dependencies: &[PathBuf],
        asset_dependencies: &[PathBuf],
        cargo_inputs: &CargoInputGraph,
    ) -> Result<(), String> {
        let configuration = WatchConfiguration::new(dependencies, asset_dependencies, cargo_inputs);
        if configuration == self.configuration {
            return Ok(());
        }
        if configuration.roots != self.configuration.roots
            || configuration.excluded_roots != self.configuration.excluded_roots
        {
            let (watcher, events) =
                create_watcher(&configuration.roots, &configuration.excluded_roots)?;
            self.watcher = watcher;
            self.events = events;
        }
        self.configuration = configuration;
        self.pending_rescan = true;
        Ok(())
    }

    pub(super) fn wait_for_change(&mut self, timeout: Duration) -> Result<bool, String> {
        if self.pending_rescan {
            self.pending_rescan = false;
            self.last_snapshot = Instant::now();
            return Ok(true);
        }

        let deadline = Instant::now() + timeout;
        loop {
            let now = Instant::now();
            if now.duration_since(self.last_snapshot) >= FULL_RESCAN_INTERVAL {
                self.last_snapshot = now;
                return Ok(true);
            }
            if now >= deadline {
                return Ok(false);
            }
            let rescan_deadline = self.last_snapshot + FULL_RESCAN_INTERVAL;
            let wait_until = deadline.min(rescan_deadline);
            let wait = wait_until.saturating_duration_since(now);
            match self.events.recv_timeout(wait) {
                Ok(Ok(event)) => {
                    self.suppress_excluded_tree(&event);
                    if self.event_requires_snapshot(&event) {
                        self.drain_events();
                        self.last_snapshot = Instant::now();
                        return Ok(true);
                    }
                }
                Ok(Err(error)) => {
                    eprintln!(
                        "ice dev: filesystem notification failed: {error}; verifying the complete input snapshot"
                    );
                    self.drain_events();
                    self.last_snapshot = Instant::now();
                    return Ok(true);
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => {
                    return Err("ice dev: filesystem notification channel closed".to_owned());
                }
            }
        }
    }

    fn drain_events(&self) {
        loop {
            match self.events.try_recv() {
                Ok(_) => {}
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => return,
            }
        }
    }

    fn event_requires_snapshot(&self, event: &Event) -> bool {
        event.need_rescan()
            || !matches!(event.kind, EventKind::Access(_))
                && (event.paths.is_empty()
                    || event
                        .paths
                        .iter()
                        .any(|path| !ignored_event_path(path, &self.configuration.excluded_roots)))
    }

    fn suppress_excluded_tree(&mut self, event: &Event) {
        if !matches!(event.kind, EventKind::Create(_)) {
            return;
        }
        for excluded in &self.configuration.excluded_roots {
            if excluded.is_dir() && event.paths.iter().any(|path| path.starts_with(excluded)) {
                let _ = self.watcher.unwatch(excluded);
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct WatchConfiguration {
    roots: Vec<WatchRoot>,
    excluded_roots: Vec<PathBuf>,
    dependencies: Vec<PathBuf>,
    asset_dependencies: Vec<PathBuf>,
    cargo_inputs: CargoInputGraph,
}

impl WatchConfiguration {
    fn new(
        dependencies: &[PathBuf],
        asset_dependencies: &[PathBuf],
        cargo_inputs: &CargoInputGraph,
    ) -> Self {
        let dependencies = sorted_paths(dependencies);
        let asset_dependencies = sorted_paths(asset_dependencies);
        let excluded_roots = watch_excluded_roots(cargo_inputs);
        let roots = watch_roots(&dependencies, &asset_dependencies, cargo_inputs);
        Self {
            roots,
            excluded_roots,
            dependencies,
            asset_dependencies,
            cargo_inputs: cargo_inputs.clone(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct WatchRoot {
    path: PathBuf,
    recursive: bool,
}

fn create_watcher(
    roots: &[WatchRoot],
    excluded_roots: &[PathBuf],
) -> Result<(RecommendedWatcher, Receiver<notify::Result<Event>>), String> {
    let (sender, events) = mpsc::channel();
    let ignored = excluded_roots.to_vec();
    let mut watcher = notify::recommended_watcher(move |result: notify::Result<Event>| {
        let forward = match &result {
            Ok(event) => event.need_rescan() || event_may_change_inputs(event, &ignored),
            Err(_) => true,
        };
        if forward {
            let _ = sender.send(result);
        }
    })
    .map_err(|error| format!("ice dev: cannot start filesystem notifications: {error}"))?;
    for root in roots {
        watcher
            .watch(
                &root.path,
                if root.recursive {
                    RecursiveMode::Recursive
                } else {
                    RecursiveMode::NonRecursive
                },
            )
            .map_err(|error| {
                format!(
                    "ice dev: cannot watch filesystem input {}: {error}",
                    root.path.display()
                )
            })?;
    }
    for excluded in excluded_roots.iter().filter(|path| path.is_dir()) {
        let _ = watcher.unwatch(excluded);
    }
    Ok((watcher, events))
}

fn event_may_change_inputs(event: &Event, excluded_roots: &[PathBuf]) -> bool {
    !matches!(event.kind, EventKind::Access(_))
        && (event.paths.is_empty()
            || event.paths.iter().any(|path| {
                !ignored_event_path(path, excluded_roots)
                    || matches!(event.kind, EventKind::Create(_)) && path.is_dir()
            }))
}

fn watch_roots(
    dependencies: &[PathBuf],
    asset_dependencies: &[PathBuf],
    cargo_inputs: &CargoInputGraph,
) -> Vec<WatchRoot> {
    let mut requested = Vec::new();
    for root in &cargo_inputs.package_roots {
        request_tree(root, &mut requested);
    }
    for path in dependencies
        .iter()
        .chain(asset_dependencies)
        .chain(&cargo_inputs.workspace_files)
    {
        request_path(path, &mut requested);
    }
    for path in &cargo_inputs.discovered_inputs {
        if path.is_dir() {
            request_tree(path, &mut requested);
        } else {
            request_path(path, &mut requested);
        }
    }
    collapse_roots(requested)
}

fn request_tree(path: &Path, requested: &mut Vec<WatchRoot>) {
    let path = normalize_watch_path(path);
    if path.is_dir() {
        requested.push(WatchRoot {
            path,
            recursive: true,
        });
    } else {
        request_missing_path(&path, requested);
    }
}

fn request_path(path: &Path, requested: &mut Vec<WatchRoot>) {
    let path = normalize_watch_path(path);
    if path.is_dir() {
        requested.push(WatchRoot {
            path,
            recursive: true,
        });
        return;
    }
    let Some(parent) = path.parent() else {
        return;
    };
    if parent.is_dir() {
        requested.push(WatchRoot {
            path: parent.to_owned(),
            recursive: false,
        });
    } else {
        request_missing_path(&path, requested);
    }
}

fn request_missing_path(path: &Path, requested: &mut Vec<WatchRoot>) {
    let mut ancestor = path.parent();
    while let Some(candidate) = ancestor {
        if candidate.is_dir() {
            if candidate.parent().is_some() {
                requested.push(WatchRoot {
                    path: candidate.to_owned(),
                    recursive: true,
                });
            }
            return;
        }
        ancestor = candidate.parent();
    }
}

fn collapse_roots(mut requested: Vec<WatchRoot>) -> Vec<WatchRoot> {
    requested.sort_by(|left, right| {
        left.path
            .components()
            .count()
            .cmp(&right.path.components().count())
            .then_with(|| left.path.cmp(&right.path))
            .then_with(|| match (left.recursive, right.recursive) {
                (true, false) => Ordering::Less,
                (false, true) => Ordering::Greater,
                _ => Ordering::Equal,
            })
    });
    let mut roots = Vec::<WatchRoot>::new();
    for request in requested {
        if roots.iter().any(|root| {
            root.path == request.path && (root.recursive || !request.recursive)
                || root.recursive && request.path.starts_with(&root.path)
        }) {
            continue;
        }
        if request.recursive {
            roots.retain(|root| !root.path.starts_with(&request.path));
        }
        roots.push(request);
    }
    roots
}

fn ignored_event_path(path: &Path, excluded_roots: &[PathBuf]) -> bool {
    excluded_roots
        .iter()
        .any(|excluded| path.starts_with(excluded))
        || path.ancestors().any(|ancestor| {
            ignored_dir(ancestor)
                || ancestor.file_name().and_then(|name| name.to_str()) == Some("vendor")
        })
}

fn sorted_paths(paths: &[PathBuf]) -> Vec<PathBuf> {
    let mut paths = paths
        .iter()
        .map(|path| normalize_watch_path(path))
        .collect::<Vec<_>>();
    paths.sort();
    paths.dedup();
    paths
}

fn watch_excluded_roots(cargo_inputs: &CargoInputGraph) -> Vec<PathBuf> {
    let mut excluded = cargo_inputs.excluded_roots.clone();
    for root in &cargo_inputs.package_roots {
        excluded.extend(
            [".git", ".worktree", "target", "vendor", "tests/cases"]
                .into_iter()
                .map(|path| root.join(path)),
        );
    }
    sorted_paths(&excluded)
}

#[cfg(test)]
mod tests {
    use super::*;
    use notify::event::{AccessKind, CreateKind, ModifyKind};
    use std::fs;

    #[test]
    fn watch_roots_collapse_nested_inputs_and_cover_missing_paths() {
        let fixture = tempfile::tempdir().unwrap();
        let external_fixture = tempfile::tempdir().unwrap();
        let root = fixture.path();
        fs::create_dir(root.join("src")).unwrap();
        fs::write(root.join("src/main.rs"), "fn main() {}\n").unwrap();
        let external = external_fixture.path().to_owned();
        let missing = external.join("nested/part.ice");
        let graph = CargoInputGraph::workspace(root);

        let configuration = WatchConfiguration::new(&[root.join("app.ice"), missing], &[], &graph);

        assert!(configuration.roots.contains(&WatchRoot {
            path: root.to_owned(),
            recursive: true,
        }));
        assert!(configuration.roots.contains(&WatchRoot {
            path: external,
            recursive: true,
        }));
        assert_eq!(configuration.roots.len(), 2);
    }

    #[test]
    fn access_and_excluded_events_do_not_trigger_snapshots() {
        let fixture = tempfile::tempdir().unwrap();
        let root = fixture.path();
        fs::create_dir(root.join("target")).unwrap();
        fs::write(root.join("Cargo.toml"), "[package]\nname='demo'\n").unwrap();
        fs::write(root.join("target/output"), "generated").unwrap();
        let graph = CargoInputGraph::workspace(root);
        let watcher = DevWatcher::new(&[], &[], &graph).unwrap();

        let access = Event {
            kind: EventKind::Access(AccessKind::Any),
            paths: vec![root.join("Cargo.toml")],
            attrs: Default::default(),
        };
        assert!(!watcher.event_requires_snapshot(&access));

        let excluded = Event {
            kind: EventKind::Modify(ModifyKind::Any),
            paths: vec![root.join("target/output")],
            attrs: Default::default(),
        };
        assert!(!watcher.event_requires_snapshot(&excluded));
        assert!(!event_may_change_inputs(
            &excluded,
            &watcher.configuration.excluded_roots
        ));

        let excluded_directory = Event {
            kind: EventKind::Create(CreateKind::Folder),
            paths: vec![root.join("target")],
            attrs: Default::default(),
        };
        assert!(event_may_change_inputs(
            &excluded_directory,
            &watcher.configuration.excluded_roots
        ));
        assert!(!watcher.event_requires_snapshot(&excluded_directory));
    }

    #[test]
    fn native_notifications_wake_without_idle_snapshot_polling() {
        let fixture = tempfile::tempdir().unwrap();
        let root = fixture.path();
        fs::create_dir(root.join("src")).unwrap();
        let source = root.join("src/main.rs");
        fs::write(&source, "fn main() {}\n").unwrap();
        let graph = CargoInputGraph::workspace(root);
        let mut watcher = DevWatcher::new(&[], &[], &graph).unwrap();

        assert!(watcher.wait_for_change(Duration::ZERO).unwrap());
        assert!(!watcher.wait_for_change(Duration::from_millis(50)).unwrap());
        fs::write(&source, "fn main() { println!(\"changed\"); }\n").unwrap();
        assert!(watcher.wait_for_change(Duration::from_secs(5)).unwrap());
    }

    #[test]
    fn configuration_changes_and_periodic_rescans_request_complete_snapshots() {
        let fixture = tempfile::tempdir().unwrap();
        let root = fixture.path();
        fs::create_dir(root.join("src")).unwrap();
        fs::write(root.join("src/main.rs"), "fn main() {}\n").unwrap();
        let graph = CargoInputGraph::workspace(root);
        let mut watcher = DevWatcher::new(&[], &[], &graph).unwrap();

        assert!(watcher.wait_for_change(Duration::ZERO).unwrap());
        watcher
            .update(&[root.join("app.ice")], &[], &graph)
            .unwrap();
        assert!(watcher.wait_for_change(Duration::ZERO).unwrap());
        assert!(!watcher.wait_for_change(Duration::ZERO).unwrap());

        watcher.last_snapshot = Instant::now() - FULL_RESCAN_INTERVAL;
        assert!(watcher.wait_for_change(Duration::ZERO).unwrap());
    }
}
