//! The catalog: every `ice:view` component in the catalog directory that
//! carries an `ice.manifest` section, read without compiling any of them.

use crate::limits::MAX_MODULE_BYTES;

/// Where the catalog looks for components: what `cargo ice bundle --target
/// wasm32-unknown-unknown` writes for this workspace.
const DEFAULT_CATALOG_DIR: &str = "target/app-store-catalog";

/// The versioned five-line section written by `export_app!`.
const MANIFEST_SECTION: &str = "ice.manifest";

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Capability {
    pub name: String,
}

/// A finite positive logical size, bounded like wire geometry. Private bits
/// keep equality and hashing exact without admitting NaN or signed zero.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PreferredSize([u32; 2]);

impl PreferredSize {
    pub fn new(width: f32, height: f32) -> Option<Self> {
        [width, height]
            .iter()
            .all(|value| value.is_finite() && *value > 0.0 && *value <= 8192.0)
            .then_some(Self([width.to_bits(), height.to_bits()]))
    }

    pub fn size(self) -> iced::Size {
        iced::Size::new(f32::from_bits(self.0[0]), f32::from_bits(self.0[1]))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CatalogEntry {
    pub id: String,
    pub name: String,
    pub description: String,
    pub capabilities: Vec<Capability>,
    pub preferred_size: Option<PreferredSize>,
    pub path: String,
    /// What the app's tile shows: the first letter of its name.
    pub mark: String,
    /// SHA-256 of the file as scanned, in hex. The library pins it at
    /// install and the loader checks it before anything runs.
    pub hash: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct StoreError {
    pub message: String,
}

/// The directory the store scans, as the status bar names it.
pub fn catalog_dir() -> String {
    std::env::var("APP_STORE_CATALOG").unwrap_or_else(|_| DEFAULT_CATALOG_DIR.to_string())
}

/// Lists every wasm module in the catalog directory that carries a manifest.
/// Reading the section needs no compilation, so a catalog of a hundred apps
/// costs a hundred file reads, not a hundred cranelift runs. A file past
/// [`MAX_MODULE_BYTES`] is left out before it is read at all, the same way a
/// bad manifest leaves a module out.
///
/// Polled by the host executor; filesystem reads never run in an Ice handler.
pub async fn scan_catalog() -> Vec<CatalogEntry> {
    scan_dir(std::path::Path::new(&catalog_dir()))
}

/// The scan itself, over a directory named directly rather than through the
/// `APP_STORE_CATALOG` env var — so a test can point it at a scratch
/// directory without touching process-global state.
pub(crate) fn scan_dir(dir: &std::path::Path) -> Vec<CatalogEntry> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut catalog: Vec<CatalogEntry> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "wasm"))
        .filter(|path| {
            std::fs::metadata(path).is_ok_and(|metadata| metadata.len() <= MAX_MODULE_BYTES)
        })
        .filter_map(|path| {
            let bytes = std::fs::read(&path).ok()?;
            let manifest = read_manifest(&bytes)?;
            let mark = manifest
                .name
                .chars()
                .next()
                .map(|first| first.to_uppercase().to_string())
                .unwrap_or_default();
            Some(CatalogEntry {
                id: path.file_stem()?.to_string_lossy().into_owned(),
                name: manifest.name,
                description: manifest.description,
                preferred_size: manifest.preferred_size,
                capabilities: manifest
                    .capabilities
                    .iter()
                    .map(|name| Capability { name: name.clone() })
                    .collect(),
                path: path.to_string_lossy().into_owned(),
                mark,
                hash: sha256_hex(&bytes),
            })
        })
        .collect();
    catalog.sort_by(|a, b| (&a.name, &a.id).cmp(&(&b.name, &b.id)));
    catalog
}

/// The content hash the catalog, the library and the loader all speak in.
pub fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::Digest;
    let digest = sha2::Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Enough of a hash to tell two builds apart on a card.
pub fn short_hash(hash: String) -> String {
    hash.chars().take(16).collect()
}

pub fn find_entry(catalog: &[CatalogEntry], id: &str) -> Option<CatalogEntry> {
    catalog.iter().find(|entry| entry.id == id).cloned()
}

/// The entries whose name, description or capabilities mention the query,
/// case-insensitively; all of them for an empty query.
pub fn filter_catalog(catalog: &[CatalogEntry], query: String) -> Vec<CatalogEntry> {
    let query = query.trim().to_lowercase();
    catalog
        .iter()
        .filter(|entry| {
            query.is_empty()
                || entry.name.to_lowercase().contains(&query)
                || entry.description.to_lowercase().contains(&query)
                || entry
                    .capabilities
                    .iter()
                    .any(|capability| capability.name.contains(&query))
        })
        .cloned()
        .collect()
}

/// What granting a capability lets the app do, in the user's terms.
pub fn capability_hint(name: String) -> String {
    match name.as_str() {
        "clipboard" => "Read and replace text in the standard and primary clipboards.",
        "clock" => "Read the host's clock, sleep, and be woken every so often.",
        "storage" => {
            "Keep up to 64 MB of its own data in the host's storage; it survives a reinstall."
        }
        "bus" => "Publish to the app bus and listen to what other apps publish.",
        _ => "A capability this store does not know; the host will refuse it.",
    }
    .to_string()
}

struct Manifest {
    name: String,
    description: String,
    capabilities: Vec<String>,
    preferred_size: Option<PreferredSize>,
}

/// What a manifest may say about itself. The catalog is read before anything
/// is installed, and the store shapes every field of every entry on every
/// relayout — outside the sandbox, with no fuel and no memory limit — so a
/// module whose manifest is a megabyte of capability names is left out of the
/// catalog rather than laid out.
const MAX_NAME_BYTES: usize = 64;
const MAX_DESCRIPTION_BYTES: usize = 256;
const MAX_CAPABILITIES: usize = 16;
const MAX_CAPABILITY_BYTES: usize = 32;

fn read_manifest(bytes: &[u8]) -> Option<Manifest> {
    // The parser walks into the core modules a component nests, which is
    // where the app's own sections are.
    let mut payloads = wasmparser::Parser::new(0).parse_all(bytes);
    // A bare core module — an app built but not yet componentized — is not
    // something the host can instantiate, so it is not in the catalog.
    let Some(Ok(wasmparser::Payload::Version {
        encoding: wasmparser::Encoding::Component,
        ..
    })) = payloads.next()
    else {
        return None;
    };
    let mut manifest = None;
    for payload in payloads {
        if let wasmparser::Payload::CustomSection(section) = payload.ok()?
            && section.name() == MANIFEST_SECTION
        {
            if manifest.is_some() {
                return None;
            }
            manifest = Some(Manifest::parse(std::str::from_utf8(section.data()).ok()?)?);
        }
    }
    manifest
}

impl Manifest {
    fn parse(text: &str) -> Option<Self> {
        if text.len() > 1024 || text.chars().any(|c| c.is_control() && c != '\n') {
            return None;
        }
        let mut lines = text.split('\n');
        if lines.next()? != "ice.manifest.v1" {
            return None;
        }
        let name = lines.next()?.to_owned();
        let description = lines.next()?.to_owned();
        let caps = lines.next()?;
        let capabilities = if caps.is_empty() {
            Vec::new()
        } else {
            let caps = caps.strip_suffix(',')?;
            if caps.split(',').any(str::is_empty) {
                return None;
            }
            caps.split(',').map(str::to_owned).collect()
        };
        let preferred_size = match lines.next()? {
            "none" => None,
            value => {
                let (width, height) = value.split_once(',')?;
                Some(PreferredSize::new(
                    width.parse().ok()?,
                    height.parse().ok()?,
                )?)
            }
        };
        if lines.next().is_some() {
            return None;
        }
        let manifest = Self {
            name,
            description,
            capabilities,
            preferred_size,
        };
        manifest.within_bounds().then_some(manifest)
    }

    fn within_bounds(&self) -> bool {
        !self.name.is_empty()
            && self.name.len() <= MAX_NAME_BYTES
            && self.description.len() <= MAX_DESCRIPTION_BYTES
            && self.capabilities.len() <= MAX_CAPABILITIES
            && self
                .capabilities
                .iter()
                .all(|capability| capability.len() <= MAX_CAPABILITY_BYTES)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;

    #[test]
    #[ignore = "requires bundled window-size-fixture wasm"]
    fn bundled_preferred_size_reaches_initial_native_open() {
        use iced::futures::{StreamExt, executor::block_on};
        use iced_test::runtime::{Action, task, window};

        let directory =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../target/window-size-fixture");
        let entries = scan_dir(&directory);
        assert_eq!(entries.len(), 1, "bundle the window-size fixture first");
        let entry = entries.into_iter().next().unwrap();
        assert_eq!(
            entry.preferred_size.unwrap().size(),
            iced::Size::new(640.5, 480.25)
        );
        let loaded = block_on(crate::store::install_app(entry)).expect("instantiate sized guest");
        let saved = crate::library::Placement {
            id: loaded.id.clone(),
            x: -12.5,
            y: 18.25,
            w: 920.5,
            h: 680.25,
            placed: true,
        };
        for (placements, expected) in [
            (vec![], iced::Size::new(640.5, 480.25)),
            (vec![saved], iced::Size::new(920.5, 680.25)),
        ] {
            let placements = crate::library::prepare_window(placements, &Some(loaded.clone()));
            let mut actions = task::into_stream(crate::library::open_guest(
                Some(loaded.clone()),
                placements.clone(),
            ))
            .unwrap();
            let Some(Action::Window(window::Action::Open(id, settings, reply))) =
                block_on(actions.next())
            else {
                panic!("first action must open the native window")
            };
            assert_eq!(
                settings.size, expected,
                "size must be correct before the first frame"
            );
            assert_eq!(settings.min_size, Some(iced::Size::new(320.0, 240.0)));
            if expected.width == 920.5 {
                assert!(
                    matches!(settings.position, iced::window::Position::Specific(p) if p == iced::Point::new(-12.5, 18.25))
                );
            }
            // Mount the actual guest at the geometry supplied to native open.
            use iced::advanced::{renderer::Headless, widget::Operation};
            struct Containers(Vec<iced::Rectangle>);
            impl Operation for Containers {
                fn traverse(&mut self, visit: &mut dyn FnMut(&mut dyn Operation)) {
                    visit(self);
                }
                fn container(&mut self, _: Option<&iced::widget::Id>, bounds: iced::Rectangle) {
                    self.0.push(bounds);
                }
            }
            let mut renderer = block_on(<iced::Renderer as Headless>::new(
                iced::Font::DEFAULT,
                iced::Pixels(16.0),
                Some("tiny-skia"),
            ))
            .expect("headless renderer");
            let mut ui = iced_test::runtime::UserInterface::build(
                crate::store::wasm_view(loaded.surface.clone(), false),
                settings.size,
                iced_test::runtime::user_interface::Cache::default(),
                &mut renderer,
            );
            for frame in 0..4 {
                ui.update(
                    &[iced::Event::Window(iced::window::Event::RedrawRequested(
                        std::time::Instant::now() + std::time::Duration::from_secs(frame),
                    ))],
                    iced::mouse::Cursor::Unavailable,
                    &mut renderer,
                    &mut iced::advanced::clipboard::Null,
                    &mut vec![],
                );
                ui = iced_test::runtime::UserInterface::build(
                    crate::store::wasm_view(loaded.surface.clone(), false),
                    settings.size,
                    ui.into_cache(),
                    &mut renderer,
                );
            }
            let mut bounds = Containers(vec![]);
            ui.operate(&renderer, &mut bounds);
            assert!(
                bounds.0.iter().any(|rect| rect.size() == expected),
                "guest fills initial native geometry: {:?}",
                bounds.0
            );
            let running = vec![crate::library::Running {
                id: loaded.id.clone(),
                name: loaded.name.clone(),
                surface: loaded.surface.clone(),
                window: id,
            }];
            if !placements[0].placed {
                let resized =
                    crate::library::resized(placements.clone(), &running, id, 700.0, 500.0);
                assert!(!resized[0].placed, "resize cannot invent a known position");
                let resized_dir = ScratchDir::new("preferred-size-resize");
                crate::library::save_placements_in(&resized, &resized_dir.0);
                let saved = std::fs::read_to_string(resized_dir.0.join("windows")).unwrap();
                assert!(!saved.is_empty(), "resize-only geometry must be persisted");
                let restored = saved
                    .lines()
                    .map(|line| crate::library::parse_placement(line).unwrap())
                    .collect();
                let mut resized_open =
                    task::into_stream(crate::library::open_guest(Some(loaded.clone()), restored))
                        .unwrap();
                let Some(Action::Window(window::Action::Open(_, settings, _))) =
                    block_on(resized_open.next())
                else {
                    panic!("resized reopen")
                };
                assert_eq!(settings.size, iced::Size::new(700.0, 500.0));
                assert!(matches!(settings.position, iced::window::Position::Default));
            }
            let moved = crate::library::moved(placements, &running, id, 25.5, 36.25);
            assert_eq!(
                (moved[0].w, moved[0].h),
                (expected.width as f64, expected.height as f64),
                "a move without a resize must preserve declared dimensions"
            );
            let saved_dir = ScratchDir::new("preferred-size-placement");
            assert!(!crate::library::save_placements_in(&moved, &saved_dir.0));
            let restored = std::fs::read_to_string(saved_dir.0.join("windows"))
                .unwrap()
                .lines()
                .map(|line| crate::library::parse_placement(line).unwrap())
                .collect();
            let mut reopen =
                task::into_stream(crate::library::open_guest(Some(loaded.clone()), restored))
                    .unwrap();
            let Some(Action::Window(window::Action::Open(_, reopened, _))) =
                block_on(reopen.next())
            else {
                panic!("reopen uses native Open")
            };
            assert_eq!(
                reopened.size, expected,
                "saved move preserves the original size on reopen"
            );
            assert!(
                matches!(reopened.position, iced::window::Position::Specific(p) if p == iced::Point::new(25.5, 36.25))
            );
            reply.send(id).expect("window opened");
            assert!(
                matches!(block_on(actions.next()), Some(Action::Output(opened)) if opened == id)
            );
            assert!(
                block_on(actions.next()).is_none(),
                "no post-open resize or move"
            );
        }
    }

    /// A scratch directory under the OS temp dir, named for the test that
    /// owns it and removed when the guard drops — nothing here survives a
    /// panic mid-test.
    struct ScratchDir(std::path::PathBuf);

    impl ScratchDir {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("app-store-catalog-test-{name}"));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).expect("scratch dir");
            Self(dir)
        }
    }

    impl Drop for ScratchDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// One real built component, copied in from the catalog `cargo ice
    /// bundle` writes — `None` when the workspace hasn't built the demo apps, so the
    /// half of the test that needs a real manifest is skipped rather than
    /// failed.
    fn a_built_component() -> Option<std::path::PathBuf> {
        std::fs::read_dir(DEFAULT_CATALOG_DIR)
            .ok()?
            .flatten()
            .map(|entry| entry.path())
            .find(|path| path.extension().is_some_and(|ext| ext == "wasm"))
    }

    // Claim: untrusted module metadata has one strict current format and a
    // finite positive bounded preferred size. Dropping those guards is Red.
    #[test]
    fn manifest_format_and_preferred_size_are_strict() {
        let good = "ice.manifest.v1\nSized\nDescription\nclock,storage,\n640.5,480.25";
        let parsed = Manifest::parse(good).unwrap();
        assert_eq!(
            parsed.preferred_size.unwrap().size(),
            iced::Size::new(640.5, 480.25)
        );
        assert_eq!(parsed.capabilities, ["clock", "storage"]);
        assert!(
            Manifest::parse("ice.manifest.v1\nDefault\n\n\nnone")
                .unwrap()
                .preferred_size
                .is_none()
        );
        for invalid in [
            "Sized\nDescription\nclock,", // no legacy format
            "ice.manifest.v2\nSized\nDescription\n\nnone",
            "ice.manifest.v1\nSized\nDescription\n\nnone\nextra",
            "ice.manifest.v1\nSized\nDescription\nclock\nnone",
            "ice.manifest.v1\nSized\nDescription\nclock,,\nnone",
        ] {
            assert!(
                Manifest::parse(invalid).is_none(),
                "accepted malformed manifest: {invalid}"
            );
        }
        for invalid in [
            "NaN,500",
            "inf,500",
            "-inf,500",
            "0,500",
            "-0,500",
            "-1,500",
            "8192.01,500",
            "1e40,500",
            "1e-50,500",
            "500,0",
            "1,2,3",
        ] {
            assert!(
                Manifest::parse(&format!("ice.manifest.v1\nSized\nDescription\n\n{invalid}"))
                    .is_none(),
                "accepted {invalid}"
            );
        }
        assert!(PreferredSize::new(8192.0, f32::MIN_POSITIVE).is_some());
        assert!(PreferredSize::new(f32::from_bits(1), 1.0).is_some());
        use std::hash::{Hash, Hasher};
        let value = PreferredSize::new(640.5, 480.25).unwrap();
        let mut hashes = [
            std::collections::hash_map::DefaultHasher::new(),
            std::collections::hash_map::DefaultHasher::new(),
        ];
        value.hash(&mut hashes[0]);
        parsed.preferred_size.unwrap().hash(&mut hashes[1]);
        assert_eq!(hashes[0].finish(), hashes[1].finish());
    }

    fn component_manifest(text: &str) -> Vec<u8> {
        fn leb(mut value: usize, out: &mut Vec<u8>) {
            while value >= 128 {
                out.push((value as u8) | 128);
                value >>= 7;
            }
            out.push(value as u8);
        }
        let mut bytes = b"\0asm\x0d\0\x01\0".to_vec();
        let mut section = vec![MANIFEST_SECTION.len() as u8];
        section.extend_from_slice(MANIFEST_SECTION.as_bytes());
        section.extend_from_slice(text.as_bytes());
        bytes.push(0);
        leb(section.len(), &mut bytes);
        bytes.extend(section);
        bytes
    }

    #[test]
    fn catalog_requires_one_valid_current_manifest() {
        let valid = component_manifest("ice.manifest.v1\nSized\nDescription\n\n640.5,480.25");
        assert_eq!(
            read_manifest(&valid)
                .unwrap()
                .preferred_size
                .unwrap()
                .size(),
            iced::Size::new(640.5, 480.25)
        );
        let mut duplicate = valid.clone();
        duplicate.extend_from_slice(&valid[8..]);
        assert!(read_manifest(&duplicate).is_none());
        let mut malformed = valid.clone();
        malformed.extend_from_slice(&[0, 127]);
        assert!(read_manifest(&malformed).is_none());
        let scratch = ScratchDir::new("preferred-size");
        std::fs::write(scratch.0.join("sized.wasm"), valid).unwrap();
        std::fs::write(
            scratch.0.join("legacy.wasm"),
            component_manifest("Legacy\nDescription\n"),
        )
        .unwrap();
        let catalog = scan_dir(&scratch.0);
        assert_eq!(catalog.len(), 1);
        assert_eq!(
            catalog[0].preferred_size.unwrap().size(),
            iced::Size::new(640.5, 480.25)
        );
    }

    #[test]
    fn catalog_scans_detect_add_change_remove_without_moving_consent_pins() {
        let scratch = ScratchDir::new("catalog_changes");
        assert!(scan_dir(&scratch.0).is_empty());
        let path = scratch.0.join("sample.wasm");
        std::fs::write(
            &path,
            component_manifest("ice.manifest.v1\nSample\nFirst build\nclock,storage,\nnone"),
        )
        .unwrap();
        let initial = scan_dir(&scratch.0);
        assert_eq!(initial.len(), 1);
        let pins = vec![crate::library::Installed {
            id: initial[0].id.clone(),
            hash: initial[0].hash.clone(),
        }];
        assert!(crate::library::pinned(&pins, &initial[0]));
        assert_eq!(scan_dir(&scratch.0), initial, "unchanged scan is equal");

        std::fs::write(
            &path,
            component_manifest("ice.manifest.v1\nSample\nSecond build\nclock,storage,\nnone"),
        )
        .unwrap();
        let changed = scan_dir(&scratch.0);
        assert_eq!(changed.len(), 1);
        assert_ne!(changed[0].hash, initial[0].hash);
        assert_eq!(changed[0].description, "Second build");
        assert!(crate::library::changed(&pins, &changed[0]));
        assert_eq!(pins[0].hash, initial[0].hash);

        std::fs::write(scratch.0.join("partial.wasm"), b"incomplete").unwrap();
        assert_eq!(scan_dir(&scratch.0), changed);
        std::fs::remove_file(path).unwrap();
        assert!(scan_dir(&scratch.0).is_empty());
    }

    #[test]
    fn equal_names_have_stable_catalog_order() {
        let scratch = ScratchDir::new("equal_names");
        for id in ["z", "a"] {
            std::fs::write(
                scratch.0.join(format!("{id}.wasm")),
                component_manifest(&format!(
                    "ice.manifest.v1\nSample\n{id}\nclock,storage,\nnone"
                )),
            )
            .unwrap();
        }
        let entries = scan_dir(&scratch.0);
        assert_eq!(
            entries
                .iter()
                .map(|entry| entry.id.as_str())
                .collect::<Vec<_>>(),
            ["a", "z"]
        );
    }

    #[test]
    fn an_oversized_module_is_left_out_of_the_catalog() {
        let scratch = ScratchDir::new("oversized_module_is_left_out");

        // A sparse file: its declared length crosses the limit, but no bytes
        // are actually written, so the test costs no disk and no time to
        // create.
        let oversized = scratch.0.join("too_big.wasm");
        let file = File::create(&oversized).expect("create oversized file");
        file.set_len(MAX_MODULE_BYTES + 1).expect("set_len");
        drop(file);

        let mut real_component_id = None;
        if let Some(built) = a_built_component() {
            let real = scratch.0.join("real.wasm");
            std::fs::copy(&built, &real).expect("copy built component");
            real_component_id = Some(real.file_stem().unwrap().to_string_lossy().into_owned());
        }

        let catalog = scan_dir(&scratch.0);

        assert!(
            catalog.iter().all(|entry| entry.id != "too_big"),
            "an oversized module must not reach the catalog"
        );
        if let Some(id) = real_component_id {
            assert!(
                catalog.iter().any(|entry| entry.id == id),
                "a component within the size limit must still be found"
            );
        }
    }
}
