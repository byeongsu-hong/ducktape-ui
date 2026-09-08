//! The catalog: every `ice:view` component in the catalog directory that
//! carries an `ice.manifest` section, read without compiling any of them.

use crate::limits::MAX_MODULE_BYTES;

/// Where the catalog looks for components: what `cargo ice bundle --target
/// wasm32-unknown-unknown` writes for this workspace.
const DEFAULT_CATALOG_DIR: &str = "target/app-store-catalog";

/// The custom section `export_app!` writes: `name\ndescription\ncap,cap,`.
const MANIFEST_SECTION: &str = "ice.manifest";

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Capability {
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CatalogEntry {
    pub id: String,
    pub name: String,
    pub description: String,
    pub capabilities: Vec<Capability>,
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
fn scan_dir(dir: &std::path::Path) -> Vec<CatalogEntry> {
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
    for payload in payloads {
        if let Ok(wasmparser::Payload::CustomSection(section)) = payload
            && section.name() == MANIFEST_SECTION
        {
            let text = std::str::from_utf8(section.data()).ok()?;
            let mut lines = text.lines();
            let name = lines.next()?.to_string();
            let description = lines.next()?.to_string();
            let capabilities = lines
                .next()
                .unwrap_or_default()
                .split(',')
                .filter(|capability| !capability.is_empty())
                .map(str::to_string)
                .collect();
            let manifest = Manifest {
                name,
                description,
                capabilities,
            };
            return manifest.within_bounds().then_some(manifest);
        }
    }
    None
}

impl Manifest {
    fn within_bounds(&self) -> bool {
        self.name.len() <= MAX_NAME_BYTES
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

    // The scanner only needs a valid component header and manifest custom section;
    // loading executable guest code is a separate boundary.
    fn component_manifest(description: &str) -> Vec<u8> {
        let manifest = format!("Sample\n{description}\nclock,storage,");
        let mut section = vec![MANIFEST_SECTION.len() as u8];
        section.extend_from_slice(MANIFEST_SECTION.as_bytes());
        section.extend_from_slice(manifest.as_bytes());
        assert!(section.len() < 128);
        let mut bytes = b"\0asm\x0d\0\x01\0".to_vec();
        bytes.extend_from_slice(&[0, section.len() as u8]);
        bytes.extend(section);
        bytes
    }

    #[test]
    fn catalog_scans_detect_add_change_remove_without_moving_consent_pins() {
        let scratch = ScratchDir::new("catalog_changes");
        assert!(scan_dir(&scratch.0).is_empty());
        let path = scratch.0.join("sample.wasm");
        std::fs::write(&path, component_manifest("First build")).unwrap();
        let initial = scan_dir(&scratch.0);
        assert_eq!(initial.len(), 1);
        let pins = vec![crate::library::Installed {
            id: initial[0].id.clone(),
            hash: initial[0].hash.clone(),
        }];
        assert!(crate::library::pinned(&pins, &initial[0]));
        assert_eq!(scan_dir(&scratch.0), initial, "unchanged scan is equal");

        std::fs::write(&path, component_manifest("Second build")).unwrap();
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
            std::fs::write(scratch.0.join(format!("{id}.wasm")), component_manifest(id)).unwrap();
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
