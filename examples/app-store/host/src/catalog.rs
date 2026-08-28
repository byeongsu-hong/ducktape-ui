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
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct StoreError {
    pub message: String,
}

/// Lists every wasm module in the catalog directory that carries a manifest.
/// Reading the section needs no compilation, so a catalog of a hundred apps
/// costs a hundred file reads, not a hundred cranelift runs.
pub fn scan_catalog() -> Vec<CatalogEntry> {
    let dir =
        std::env::var("APP_STORE_CATALOG").unwrap_or_else(|_| DEFAULT_CATALOG_DIR.to_string());
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
            })
        })
        .collect();
    catalog.sort_by(|a, b| a.name.cmp(&b.name));
    catalog
}

struct Manifest {
    name: String,
    description: String,
    capabilities: Vec<String>,
}

/// What a manifest may say about itself. The catalog is read before anything
/// is installed, and the sidebar shapes every field of every entry on every
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
