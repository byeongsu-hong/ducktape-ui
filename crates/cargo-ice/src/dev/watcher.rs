use super::inputs::{CargoInputGraph, build_input_files, normalize_watch_path};
use crate::ignored_dir;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, SyncSender, TrySendError};
use std::thread;
use std::time::{Duration, Instant, SystemTime};

const FULL_RESCAN_INTERVAL: Duration = Duration::from_secs(30);
const POLLING_INTERVAL: Duration = Duration::from_millis(750);
/// Hold a routine native save burst; sustained overflow falls back to the
/// complete content-stamp snapshot instead of retaining paths without a bound.
const NATIVE_EVENT_BUFFER_CAPACITY: usize = 64;
const POLLING_FALLBACK_MESSAGE: &str =
    "native notifications unavailable; using polling safety mode";

#[derive(Debug, Eq, PartialEq)]
pub(super) enum DevChange {
    Paths(Vec<PathBuf>),
    FullRescan,
}

pub(super) struct DevWatcher {
    backend: WatchBackend,
    configuration: WatchConfiguration,
    pending_rescan: bool,
    last_full_rescan: Instant,
}

enum WatchBackend {
    Native(NativeWatcher),
    Polling(MetadataPoller),
}

struct NativeWatcher {
    watcher: RecommendedWatcher,
    events: Receiver<notify::Result<Event>>,
    overflowed: Arc<AtomicBool>,
}

struct MetadataPoller {
    observed: MetadataSnapshot,
    next_poll: Instant,
}

type MetadataSnapshot = Vec<(PathBuf, MetadataState)>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MetadataState {
    Missing,
    Unreadable,
    Present {
        is_file: bool,
        is_directory: bool,
        len: u64,
        modified: Option<SystemTime>,
    },
}

impl DevWatcher {
    pub(super) fn new(
        dependencies: &[PathBuf],
        asset_dependencies: &[PathBuf],
        cargo_inputs: &CargoInputGraph,
    ) -> Self {
        Self::new_with_native_factory(
            dependencies,
            asset_dependencies,
            cargo_inputs,
            create_native_watcher,
        )
    }

    fn new_with_native_factory(
        dependencies: &[PathBuf],
        asset_dependencies: &[PathBuf],
        cargo_inputs: &CargoInputGraph,
        create_native: impl FnOnce(&[WatchRoot], &[PathBuf]) -> Result<NativeWatcher, String>,
    ) -> Self {
        let configuration = WatchConfiguration::new(dependencies, asset_dependencies, cargo_inputs);
        let backend = native_or_polling(&configuration, create_native);
        Self {
            backend,
            configuration,
            pending_rescan: true,
            last_full_rescan: Instant::now(),
        }
    }

    pub(super) fn update(
        &mut self,
        dependencies: &[PathBuf],
        asset_dependencies: &[PathBuf],
        cargo_inputs: &CargoInputGraph,
    ) {
        let configuration = WatchConfiguration::new(dependencies, asset_dependencies, cargo_inputs);
        if configuration == self.configuration {
            return;
        }
        if configuration.roots != self.configuration.roots
            || configuration.excluded_roots != self.configuration.excluded_roots
        {
            self.backend = if matches!(&self.backend, WatchBackend::Native(_)) {
                native_or_polling(&configuration, create_native_watcher)
            } else {
                WatchBackend::Polling(MetadataPoller::new(&configuration))
            };
        } else if let WatchBackend::Polling(poller) = &mut self.backend {
            poller.refresh(&configuration);
        }
        self.configuration = configuration;
        self.pending_rescan = true;
    }

    pub(super) fn wait_for_change(&mut self, timeout: Duration) -> Option<DevChange> {
        if self.pending_rescan {
            self.pending_rescan = false;
            self.last_full_rescan = Instant::now();
            return Some(DevChange::FullRescan);
        }

        let deadline = Instant::now() + timeout;
        loop {
            let now = Instant::now();
            if now.duration_since(self.last_full_rescan) >= FULL_RESCAN_INTERVAL {
                self.last_full_rescan = now;
                return Some(DevChange::FullRescan);
            }
            let rescan_deadline = self.last_full_rescan + FULL_RESCAN_INTERVAL;
            match &mut self.backend {
                WatchBackend::Native(native) => {
                    if now >= deadline {
                        return None;
                    }
                    let wait_until = deadline.min(rescan_deadline);
                    let wait = wait_until.saturating_duration_since(now);
                    match native.wait_for_change(wait, &self.configuration.excluded_roots) {
                        Ok(Some(DevChange::FullRescan)) => {
                            self.last_full_rescan = Instant::now();
                            return Some(DevChange::FullRescan);
                        }
                        Ok(change @ Some(DevChange::Paths(_))) => return change,
                        Ok(None) => {}
                        Err(RecvTimeoutError::Timeout) => {}
                        Err(RecvTimeoutError::Disconnected) => {
                            switch_to_polling(&mut self.backend, &self.configuration);
                            self.last_full_rescan = Instant::now();
                            return Some(DevChange::FullRescan);
                        }
                    }
                }
                WatchBackend::Polling(poller) => {
                    if poller.poll_if_due(now, &self.configuration) {
                        self.last_full_rescan = now;
                        return Some(DevChange::FullRescan);
                    }
                    if now >= deadline {
                        return None;
                    }
                    let wait_until = deadline.min(rescan_deadline).min(poller.next_poll);
                    thread::sleep(wait_until.saturating_duration_since(now));
                }
            }
        }
    }
}

impl NativeWatcher {
    fn wait_for_change(
        &mut self,
        wait: Duration,
        excluded_roots: &[PathBuf],
    ) -> Result<Option<DevChange>, RecvTimeoutError> {
        let first = self.events.recv_timeout(wait)?;
        let mut paths = BTreeSet::new();
        let mut full_rescan = self.collect_change(first, excluded_roots, &mut paths);
        for _ in 1..NATIVE_EVENT_BUFFER_CAPACITY {
            let Ok(result) = self.events.try_recv() else {
                break;
            };
            full_rescan |= self.collect_change(result, excluded_roots, &mut paths);
        }
        if self.overflowed.swap(false, AtomicOrdering::Relaxed) {
            for _ in 0..NATIVE_EVENT_BUFFER_CAPACITY {
                if self.events.try_recv().is_err() {
                    break;
                }
            }
            return Ok(Some(DevChange::FullRescan));
        }
        if full_rescan {
            Ok(Some(DevChange::FullRescan))
        } else if paths.is_empty() {
            Ok(None)
        } else {
            Ok(Some(DevChange::Paths(paths.into_iter().collect())))
        }
    }

    fn collect_change(
        &mut self,
        result: notify::Result<Event>,
        excluded_roots: &[PathBuf],
        paths: &mut BTreeSet<PathBuf>,
    ) -> bool {
        let event = match result {
            Ok(event) => event,
            Err(error) => {
                tracing::warn!(
                    %error,
                    "filesystem notification failed; verifying the complete input snapshot"
                );
                return true;
            }
        };
        self.suppress_excluded_tree(&event, excluded_roots);
        if event.need_rescan() || event.paths.is_empty() {
            return true;
        }
        if matches!(event.kind, EventKind::Access(_)) {
            return false;
        }
        paths.extend(
            event
                .paths
                .into_iter()
                .filter(|path| !ignored_event_path(path, excluded_roots))
                .map(|path| normalize_watch_path(&path)),
        );
        false
    }

    fn suppress_excluded_tree(&mut self, event: &Event, excluded_roots: &[PathBuf]) {
        if !matches!(event.kind, EventKind::Create(_)) {
            return;
        }
        for excluded in excluded_roots {
            if excluded.is_dir() && event.paths.iter().any(|path| path.starts_with(excluded)) {
                let _ = self.watcher.unwatch(excluded);
            }
        }
    }
}

impl MetadataPoller {
    fn new(configuration: &WatchConfiguration) -> Self {
        Self {
            observed: metadata_snapshot(configuration),
            next_poll: Instant::now() + POLLING_INTERVAL,
        }
    }

    fn refresh(&mut self, configuration: &WatchConfiguration) {
        self.observed = metadata_snapshot(configuration);
        self.next_poll = Instant::now() + POLLING_INTERVAL;
    }

    fn poll_if_due(&mut self, now: Instant, configuration: &WatchConfiguration) -> bool {
        if now < self.next_poll {
            return false;
        }
        self.next_poll = now + POLLING_INTERVAL;
        let next = metadata_snapshot(configuration);
        if next == self.observed {
            return false;
        }
        self.observed = next;
        true
    }
}

fn native_or_polling(
    configuration: &WatchConfiguration,
    create_native: impl FnOnce(&[WatchRoot], &[PathBuf]) -> Result<NativeWatcher, String>,
) -> WatchBackend {
    match create_native(&configuration.roots, &configuration.excluded_roots) {
        Ok(watcher) => WatchBackend::Native(watcher),
        Err(_) => {
            tracing::warn!("{POLLING_FALLBACK_MESSAGE}");
            WatchBackend::Polling(MetadataPoller::new(configuration))
        }
    }
}

fn switch_to_polling(backend: &mut WatchBackend, configuration: &WatchConfiguration) {
    tracing::warn!("{POLLING_FALLBACK_MESSAGE}");
    *backend = WatchBackend::Polling(MetadataPoller::new(configuration));
}

fn metadata_snapshot(configuration: &WatchConfiguration) -> MetadataSnapshot {
    let mut paths = configuration.dependencies.clone();
    paths.extend(build_input_files(
        &configuration.cargo_inputs,
        &configuration.asset_dependencies,
    ));
    paths.sort();
    paths.dedup();
    paths
        .into_iter()
        .map(|path| {
            let state = match fs::metadata(&path) {
                Ok(metadata) => MetadataState::Present {
                    is_file: metadata.is_file(),
                    is_directory: metadata.is_dir(),
                    len: metadata.len(),
                    modified: metadata.modified().ok(),
                },
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    MetadataState::Missing
                }
                Err(_) => MetadataState::Unreadable,
            };
            (path, state)
        })
        .collect()
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

fn create_native_watcher(
    roots: &[WatchRoot],
    excluded_roots: &[PathBuf],
) -> Result<NativeWatcher, String> {
    let (sender, events) = mpsc::sync_channel(NATIVE_EVENT_BUFFER_CAPACITY);
    let overflowed = Arc::new(AtomicBool::new(false));
    let callback_overflowed = Arc::clone(&overflowed);
    let ignored = excluded_roots.to_vec();
    let mut watcher = notify::recommended_watcher(move |result: notify::Result<Event>| {
        let forward = match &result {
            Ok(event) => event.need_rescan() || event_may_change_inputs(event, &ignored),
            Err(_) => true,
        };
        if forward {
            forward_native_event(&sender, &callback_overflowed, result);
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
    Ok(NativeWatcher {
        watcher,
        events,
        overflowed,
    })
}

fn forward_native_event(
    sender: &SyncSender<notify::Result<Event>>,
    overflowed: &AtomicBool,
    result: notify::Result<Event>,
) {
    match sender.try_send(result) {
        Ok(()) => {}
        Err(TrySendError::Full(result)) => record_native_overflow(sender, overflowed, result),
        Err(TrySendError::Disconnected(_)) => {}
    }
}

fn record_native_overflow(
    sender: &SyncSender<notify::Result<Event>>,
    overflowed: &AtomicBool,
    result: notify::Result<Event>,
) {
    overflowed.store(true, AtomicOrdering::Relaxed);
    let _ = sender.try_send(result);
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
    for path in &mut excluded {
        *path = normalize_watch_path(path);
    }
    excluded.sort();
    excluded.dedup();
    excluded
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dev::inputs::{
        dev_stamps_with_cargo_inputs, file_stamp_attempts, reset_file_stamp_attempts,
        settled_dev_stamps_with_cargo_inputs,
    };
    use notify::event::{AccessKind, CreateKind, ModifyKind};
    use std::fs;

    fn fallback_watcher(
        dependencies: &[PathBuf],
        asset_dependencies: &[PathBuf],
        cargo_inputs: &CargoInputGraph,
        failure: &str,
    ) -> DevWatcher {
        DevWatcher::new_with_native_factory(
            dependencies,
            asset_dependencies,
            cargo_inputs,
            |_, _| Err(failure.to_owned()),
        )
    }

    fn force_poll(watcher: &mut DevWatcher) {
        let WatchBackend::Polling(poller) = &mut watcher.backend else {
            panic!("expected polling fallback");
        };
        poller.next_poll = Instant::now();
    }

    fn expect_full_rescan(watcher: &mut DevWatcher) {
        assert_eq!(
            watcher.wait_for_change(Duration::ZERO),
            Some(DevChange::FullRescan)
        );
    }

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
    #[ignore = "allocation contract; run alone with --test-threads=1"]
    fn performance_contract_excluded_root_normalization_reuses_storage() {
        const ROOTS: usize = 128;
        const REFRESHES: usize = 256;
        const MAX_BLOCKS: u64 = 131_328;
        const MAX_BYTES: u64 = 3_609_088;

        let mut graph = CargoInputGraph::workspace(Path::new("/workspace"));
        graph.package_roots.clear();
        graph.excluded_roots = (0..ROOTS)
            .map(|index| PathBuf::from(format!("/workspace/package-{index}/./target")))
            .collect();
        drop(watch_excluded_roots(&graph));

        let _profiler = dhat::Profiler::builder().testing().build();
        for _ in 0..REFRESHES {
            std::hint::black_box(watch_excluded_roots(std::hint::black_box(&graph)));
        }
        let heap = dhat::HeapStats::get();

        eprintln!(
            "{REFRESHES} excluded-root refreshes: {} heap blocks / {} bytes",
            heap.total_blocks, heap.total_bytes
        );
        assert!(
            heap.total_blocks <= MAX_BLOCKS,
            "excluded-root refreshes allocated too many blocks: {heap:?}"
        );
        assert!(
            heap.total_bytes <= MAX_BYTES,
            "excluded-root refreshes allocated too many bytes: {heap:?}"
        );
    }

    #[test]
    fn access_and_excluded_events_do_not_trigger_snapshots() {
        let fixture = tempfile::tempdir().unwrap();
        let root = fixture.path();
        fs::create_dir(root.join("target")).unwrap();
        fs::write(root.join("Cargo.toml"), "[package]\nname='demo'\n").unwrap();
        fs::write(root.join("target/output"), "generated").unwrap();
        let graph = CargoInputGraph::workspace(root);
        let mut watcher = DevWatcher::new(&[], &[], &graph);
        let mut paths = BTreeSet::new();
        let excluded_roots = watcher.configuration.excluded_roots.clone();
        let WatchBackend::Native(native) = &mut watcher.backend else {
            panic!("native watcher unexpectedly unavailable in native-path test");
        };

        let access = Event {
            kind: EventKind::Access(AccessKind::Any),
            paths: vec![root.join("Cargo.toml")],
            attrs: Default::default(),
        };
        assert!(!native.collect_change(Ok(access), &excluded_roots, &mut paths));
        assert!(paths.is_empty());

        let excluded = Event {
            kind: EventKind::Modify(ModifyKind::Any),
            paths: vec![root.join("target/output")],
            attrs: Default::default(),
        };
        assert!(!event_may_change_inputs(&excluded, &excluded_roots));
        assert!(!native.collect_change(Ok(excluded), &excluded_roots, &mut paths));
        assert!(paths.is_empty());

        let excluded_directory = Event {
            kind: EventKind::Create(CreateKind::Folder),
            paths: vec![root.join("target")],
            attrs: Default::default(),
        };
        assert!(event_may_change_inputs(
            &excluded_directory,
            &excluded_roots
        ));
        assert!(!native.collect_change(Ok(excluded_directory), &excluded_roots, &mut paths));
        assert!(paths.is_empty());
    }

    #[test]
    fn native_notifications_wake_without_idle_snapshot_polling() {
        let fixture = tempfile::tempdir().unwrap();
        let root = fixture.path();
        fs::create_dir(root.join("src")).unwrap();
        let source = root.join("src/main.rs");
        fs::write(&source, "fn main() {}\n").unwrap();
        let graph = CargoInputGraph::workspace(root);
        let mut watcher = DevWatcher::new(&[], &[], &graph);

        assert!(matches!(&watcher.backend, WatchBackend::Native(_)));
        reset_file_stamp_attempts();

        assert_eq!(
            watcher.wait_for_change(Duration::ZERO),
            Some(DevChange::FullRescan)
        );
        assert_eq!(watcher.wait_for_change(Duration::from_millis(50)), None);
        let last_full_rescan = watcher.last_full_rescan;
        fs::write(&source, "fn main() { println!(\"changed\"); }\n").unwrap();
        let change = watcher.wait_for_change(Duration::from_secs(5)).unwrap();
        let DevChange::Paths(paths) = change else {
            panic!("source edit unexpectedly requested a complete rescan");
        };
        assert!(paths.contains(&source));
        assert_eq!(watcher.last_full_rescan, last_full_rescan);
        assert_eq!(
            file_stamp_attempts(),
            0,
            "native watcher waits must not perform content-stamp reads"
        );
    }

    #[test]
    fn native_notification_overflow_requests_one_complete_rescan() {
        let fixture = tempfile::tempdir().unwrap();
        let root = fixture.path();
        fs::create_dir(root.join("src")).unwrap();

        let event = |name: &str| {
            Ok(Event {
                kind: EventKind::Modify(ModifyKind::Any),
                paths: vec![root.join("src").join(name)],
                attrs: Default::default(),
            })
        };
        let (race_sender, race_events) = mpsc::sync_channel(1);
        race_sender.try_send(event("queued.rs")).unwrap();
        let late = match race_sender.try_send(event("late.rs")) {
            Err(TrySendError::Full(result)) => result,
            result => panic!("expected a full native event queue, got {result:?}"),
        };
        let _ = race_events.try_recv().unwrap();
        let race_overflowed = AtomicBool::new(false);
        record_native_overflow(&race_sender, &race_overflowed, late);
        assert!(race_overflowed.load(AtomicOrdering::Relaxed));
        assert_eq!(
            race_events.try_recv().unwrap().unwrap().paths,
            vec![root.join("src/late.rs")],
            "an event that overflows just before the queue drains must wake the consumer"
        );

        let graph = CargoInputGraph::workspace(root);
        let mut watcher = DevWatcher::new(&[], &[], &graph);
        expect_full_rescan(&mut watcher);

        let (sender, events) = mpsc::sync_channel(NATIVE_EVENT_BUFFER_CAPACITY);
        let overflowed = Arc::new(AtomicBool::new(false));
        let WatchBackend::Native(native) = &mut watcher.backend else {
            panic!("native watcher unexpectedly unavailable in native-path test");
        };
        native.events = events;
        native.overflowed = Arc::clone(&overflowed);

        for index in 0..=NATIVE_EVENT_BUFFER_CAPACITY {
            forward_native_event(
                &sender,
                &overflowed,
                Ok(Event {
                    kind: EventKind::Modify(ModifyKind::Any),
                    paths: vec![root.join("src").join(format!("{index}.rs"))],
                    attrs: Default::default(),
                }),
            );
        }

        assert_eq!(
            watcher.wait_for_change(Duration::from_millis(50)),
            Some(DevChange::FullRescan),
            "a dropped detail event must fall back to the complete content snapshot"
        );
        assert_eq!(
            watcher.wait_for_change(Duration::ZERO),
            None,
            "the overflow marker and queued detail must be consumed together"
        );
    }

    #[test]
    fn native_notification_batches_have_fixed_work_budgets() {
        let fixture = tempfile::tempdir().unwrap();
        let root = fixture.path();
        fs::create_dir(root.join("src")).unwrap();
        let graph = CargoInputGraph::workspace(root);
        let mut watcher = DevWatcher::new(&[], &[], &graph);
        expect_full_rescan(&mut watcher);

        let event = |index: usize| {
            Ok(Event {
                kind: EventKind::Modify(ModifyKind::Any),
                paths: vec![root.join("src").join(format!("{index}.rs"))],
                attrs: Default::default(),
            })
        };
        let (sender, events) = mpsc::sync_channel(NATIVE_EVENT_BUFFER_CAPACITY + 1);
        for index in 0..=NATIVE_EVENT_BUFFER_CAPACITY {
            sender.try_send(event(index)).unwrap();
        }
        let WatchBackend::Native(native) = &mut watcher.backend else {
            panic!("native watcher unexpectedly unavailable in native-path test");
        };
        native.events = events;
        native.overflowed.store(false, AtomicOrdering::Relaxed);

        let Some(DevChange::Paths(first)) = watcher.wait_for_change(Duration::from_millis(50))
        else {
            panic!("the first fixed native batch must preserve path detail");
        };
        assert_eq!(first.len(), NATIVE_EVENT_BUFFER_CAPACITY);
        let Some(DevChange::Paths(second)) = watcher.wait_for_change(Duration::from_millis(50))
        else {
            panic!("the event beyond the first batch must remain for the next wait");
        };
        assert_eq!(second.len(), 1);

        let queued = NATIVE_EVENT_BUFFER_CAPACITY * 2 + 1;
        let (overflow_sender, overflow_events) = mpsc::sync_channel(queued);
        for index in 0..queued {
            overflow_sender.try_send(event(index)).unwrap();
        }
        let WatchBackend::Native(native) = &mut watcher.backend else {
            panic!("native watcher unexpectedly unavailable in native-path test");
        };
        native.events = overflow_events;
        native.overflowed.store(true, AtomicOrdering::Relaxed);

        assert_eq!(
            watcher.wait_for_change(Duration::from_millis(50)),
            Some(DevChange::FullRescan)
        );
        let Some(DevChange::Paths(remaining)) = watcher.wait_for_change(Duration::from_millis(50))
        else {
            panic!("overflow cleanup must leave work beyond its fixed discard budget queued");
        };
        assert_eq!(remaining.len(), 1);
    }

    #[test]
    fn configuration_changes_and_periodic_rescans_request_complete_snapshots() {
        let fixture = tempfile::tempdir().unwrap();
        let root = fixture.path();
        fs::create_dir(root.join("src")).unwrap();
        fs::write(root.join("src/main.rs"), "fn main() {}\n").unwrap();
        let graph = CargoInputGraph::workspace(root);
        let mut watcher = DevWatcher::new(&[], &[], &graph);

        assert_eq!(
            watcher.wait_for_change(Duration::ZERO),
            Some(DevChange::FullRescan)
        );
        watcher.update(&[root.join("app.ice")], &[], &graph);
        assert_eq!(
            watcher.wait_for_change(Duration::ZERO),
            Some(DevChange::FullRescan)
        );
        assert_eq!(watcher.wait_for_change(Duration::ZERO), None);

        watcher.last_full_rescan = Instant::now() - FULL_RESCAN_INTERVAL;
        assert_eq!(
            watcher.wait_for_change(Duration::ZERO),
            Some(DevChange::FullRescan)
        );
    }

    #[test]
    fn native_creation_failures_select_polling_safety_mode() {
        let fixture = tempfile::tempdir().unwrap();
        let root = fixture.path();
        fs::create_dir(root.join("src")).unwrap();
        fs::write(root.join("src/main.rs"), "fn main() {}\n").unwrap();
        let graph = CargoInputGraph::workspace(root);

        assert_eq!(
            POLLING_FALLBACK_MESSAGE,
            "native notifications unavailable; using polling safety mode"
        );
        assert!(POLLING_INTERVAL >= Duration::from_millis(500));
        assert!(POLLING_INTERVAL <= Duration::from_secs(1));
        for failure in [
            "network filesystem notifications unavailable",
            "Function not implemented (os error 38)",
            "No space left on device (os error 28)",
        ] {
            let watcher = fallback_watcher(&[], &[], &graph, failure);
            assert!(matches!(watcher.backend, WatchBackend::Polling(_)));
        }
    }

    #[test]
    fn disconnected_native_channel_switches_to_polling_and_rescans() {
        let fixture = tempfile::tempdir().unwrap();
        let root = fixture.path();
        fs::create_dir(root.join("src")).unwrap();
        fs::write(root.join("src/main.rs"), "fn main() {}\n").unwrap();
        let graph = CargoInputGraph::workspace(root);
        let mut watcher = DevWatcher::new(&[], &[], &graph);
        expect_full_rescan(&mut watcher);

        let WatchBackend::Native(native) = &mut watcher.backend else {
            panic!("native watcher unexpectedly unavailable in native-path test");
        };
        let (sender, disconnected) = mpsc::channel();
        drop(sender);
        native.events = disconnected;

        assert_eq!(
            watcher.wait_for_change(Duration::from_millis(50)),
            Some(DevChange::FullRescan)
        );
        assert!(matches!(watcher.backend, WatchBackend::Polling(_)));
    }

    #[test]
    fn polling_fallback_detects_import_and_build_input_lifecycle() {
        let fixture = tempfile::tempdir().unwrap();
        let root = fixture.path();
        let source = root.join("app.ice");
        let fragment = root.join("fragment.ice");
        let missing_import = root.join("created-later.ice");
        let rust_source = root.join("src/main.rs");
        let added_rust = root.join("src/added.rs");
        fs::create_dir(root.join("src")).unwrap();
        fs::write(&source, "app Demo\nview\n  text \"ready\"\n").unwrap();
        fs::write(&fragment, "component Part()\n  text \"first\"\n").unwrap();
        fs::write(&rust_source, "fn main() {}\n").unwrap();
        let graph = CargoInputGraph::workspace(root);
        let mut dependencies = vec![source.clone(), fragment.clone()];
        let mut current = dev_stamps_with_cargo_inputs(root, &dependencies, &[], &graph);
        let mut watcher = fallback_watcher(
            &dependencies,
            &[],
            &graph,
            "No space left on device (os error 28)",
        );
        expect_full_rescan(&mut watcher);

        reset_file_stamp_attempts();
        fs::write(
            &fragment,
            "component Part()\n  text \"changed and longer\"\n",
        )
        .unwrap();
        force_poll(&mut watcher);
        expect_full_rescan(&mut watcher);
        assert_eq!(
            file_stamp_attempts(),
            0,
            "metadata polling must leave content verification to the existing stamp path"
        );
        current = settled_dev_stamps_with_cargo_inputs(
            root,
            &dependencies,
            &[],
            &graph,
            &current.0,
            &current.1,
        )
        .expect("changed import should pass two content-stamp reads");
        assert!(file_stamp_attempts() > 0);

        dependencies.push(missing_import.clone());
        watcher.update(&dependencies, &[], &graph);
        expect_full_rescan(&mut watcher);
        current = settled_dev_stamps_with_cargo_inputs(
            root,
            &dependencies,
            &[],
            &graph,
            &current.0,
            &current.1,
        )
        .expect("an import graph update should add the missing source to the snapshot");
        fs::write(
            &missing_import,
            "component CreatedLater()\n  text \"created\"\n",
        )
        .unwrap();
        force_poll(&mut watcher);
        expect_full_rescan(&mut watcher);
        current = settled_dev_stamps_with_cargo_inputs(
            root,
            &dependencies,
            &[],
            &graph,
            &current.0,
            &current.1,
        )
        .expect("creation of a missing imported source should be detected");
        fs::remove_file(&missing_import).unwrap();
        force_poll(&mut watcher);
        expect_full_rescan(&mut watcher);
        current = settled_dev_stamps_with_cargo_inputs(
            root,
            &dependencies,
            &[],
            &graph,
            &current.0,
            &current.1,
        )
        .expect("deletion of an imported source should be detected");

        fs::write(&rust_source, "fn main() { println!(\"changed\"); }\n").unwrap();
        force_poll(&mut watcher);
        expect_full_rescan(&mut watcher);
        current = settled_dev_stamps_with_cargo_inputs(
            root,
            &dependencies,
            &[],
            &graph,
            &current.0,
            &current.1,
        )
        .expect("changed Rust input should be detected");
        fs::write(&added_rust, "pub const ADDED: bool = true;\n").unwrap();
        force_poll(&mut watcher);
        expect_full_rescan(&mut watcher);
        current = settled_dev_stamps_with_cargo_inputs(
            root,
            &dependencies,
            &[],
            &graph,
            &current.0,
            &current.1,
        )
        .expect("new Rust input should refresh the build inventory");
        assert!(current.1.iter().any(|(path, _)| path == &added_rust));
        fs::remove_file(&added_rust).unwrap();
        force_poll(&mut watcher);
        expect_full_rescan(&mut watcher);
        let removed = settled_dev_stamps_with_cargo_inputs(
            root,
            &dependencies,
            &[],
            &graph,
            &current.0,
            &current.1,
        )
        .expect("removed Rust input should refresh the build inventory");
        assert!(removed.1.iter().all(|(path, _)| path != &added_rust));
    }
}
