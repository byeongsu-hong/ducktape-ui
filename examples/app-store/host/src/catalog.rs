//! The catalog: every wasm module in the catalog directory that carries an
//! `ice.manifest` section, read without compiling any of them.

/// Where the catalog looks for modules. Build the apps first:
/// `cargo build -p app-store-todo -p app-store-counter -p app-store-clock -p app-store-activity -p app-store-chaos --release --target wasm32-unknown-unknown`.
const DEFAULT_CATALOG_DIR: &str = "target/wasm32-unknown-unknown/release";

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
/// costs a hundred file reads, not a hundred cranelift runs.
pub fn scan_catalog() -> Vec<CatalogEntry> {
    let dir = catalog_dir();
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut catalog: Vec<CatalogEntry> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "wasm"))
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
            })
        })
        .collect();
    catalog.sort_by(|a, b| a.name.cmp(&b.name));
    catalog
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
    for payload in wasmparser::Parser::new(0).parse_all(bytes) {
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
