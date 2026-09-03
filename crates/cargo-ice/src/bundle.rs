//! Application bundling for the three desktop platforms.
//!
//! `cargo ice bundle -p <package>` builds the package in release and hands the
//! binary to the packager the host knows how to run: a signed, notarized
//! `.dmg` on macOS, a `.deb` on Linux, a per-user `.msi` on Windows.
//!
//! Identity comes from what the project already declares. The Ice `app`
//! declaration names the application and its `id`; the Cargo manifest carries
//! the version, description, authors, and homepage. Only what none of those
//! can express — the icon and the per-platform store fields — lives in
//! `[package.metadata.ice.bundle]`.

mod icon;
mod linux;
mod macos;
mod windows;

use serde_json::Value;
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const USAGE: &str = "cargo ice bundle -p <package> [--target <triple>]...";
const DEFAULT_MINIMUM_SYSTEM_VERSION: &str = "11.0";

/// macOS kills an app the moment it reaches a protected resource whose reason
/// the bundle does not declare, so the sentence the prompt shows is part of
/// the package rather than the code. Apple's list is long and keeps growing,
/// so the two a desktop app almost always needs get a shorthand and every
/// other key is written out.
const USAGE_KEY_SUFFIX: &str = "UsageDescription";
const USAGE_KEY_PREFIX: &str = "NS";
const USAGE_ALIASES: [(&str, &str); 2] = [
    ("camera", "NSCameraUsageDescription"),
    ("microphone", "NSMicrophoneUsageDescription"),
];

pub(crate) fn run(root: &Path, args: &[String]) -> Result<(), String> {
    let request = Request::parse(args)?;
    let platform = Platform::host()?;
    if platform != Platform::MacOs && request.targets.len() > 1 {
        return Err(
            "only macOS joins several --target builds into one artifact; name at most one".into(),
        );
    }
    let package = Package::resolve(&crate::dev::metadata(root)?, &request.package, platform)?;
    let meta = BundleMeta::resolve(&package, platform)?;

    let build = build_arguments(&request);
    crate::cargo(&build.iter().map(String::as_str).collect::<Vec<_>>())?;

    let output = package.target_directory.join("ice-bundle");
    create_dir(&output)?;
    let executable = link_executable(&package, &request, &output)?;
    let arch = request.arch_label();
    let source = package.icon.as_deref();
    let built = match platform {
        Platform::MacOs => macos::bundle(&output, &meta, &executable, source, &arch)?,
        Platform::Linux => linux::bundle(&output, &meta, &executable, source, &arch)?,
        Platform::Windows => windows::bundle(&output, &meta, &executable, source, &arch)?,
    };
    for artifact in &built {
        println!("{}", artifact.display());
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Platform {
    MacOs,
    Linux,
    Windows,
}

impl Platform {
    fn host() -> Result<Self, String> {
        match env::consts::OS {
            "macos" => Ok(Self::MacOs),
            "linux" => Ok(Self::Linux),
            "windows" => Ok(Self::Windows),
            other => Err(format!(
                "cargo ice bundle packages for macOS, Linux, and Windows; this is `{other}`"
            )),
        }
    }

    fn executable_suffix(self) -> &'static str {
        if matches!(self, Self::Windows) {
            ".exe"
        } else {
            ""
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
struct Request {
    package: String,
    targets: Vec<String>,
}

impl Request {
    fn parse(args: &[String]) -> Result<Self, String> {
        let mut package = None;
        let mut targets = Vec::new();
        let mut remaining = args.iter();
        while let Some(argument) = remaining.next() {
            let mut value = || {
                remaining
                    .next()
                    .cloned()
                    .ok_or_else(|| format!("`{argument}` needs a value; {USAGE}"))
            };
            match argument.as_str() {
                "-p" | "--package" => package = Some(value()?),
                "--target" => targets.push(value()?),
                _ => return Err(format!("unexpected argument `{argument}`; {USAGE}")),
            }
        }
        let package = package.ok_or_else(|| USAGE.to_owned())?;
        if targets
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            != targets.len()
        {
            return Err("cargo ice bundle was given the same --target twice".into());
        }
        Ok(Self { package, targets })
    }

    /// Names the slice of hardware the artifact runs on, so two architectures
    /// published from one release do not collide on one file name.
    fn arch_label(&self) -> String {
        match self.targets.as_slice() {
            [] => env::consts::ARCH.to_owned(),
            [target] => target
                .split('-')
                .next()
                .unwrap_or(target.as_str())
                .to_owned(),
            _ => "universal".to_owned(),
        }
    }
}

fn build_arguments(request: &Request) -> Vec<String> {
    let mut arguments = vec![
        "build".to_owned(),
        "--release".to_owned(),
        "--locked".to_owned(),
        "-p".to_owned(),
        request.package.clone(),
    ];
    for target in &request.targets {
        arguments.push("--target".to_owned());
        arguments.push(target.clone());
    }
    arguments
}

#[derive(Debug)]
struct Package {
    root: PathBuf,
    name: String,
    version: String,
    description: Option<String>,
    authors: Vec<String>,
    homepage: Option<String>,
    executable: String,
    source: PathBuf,
    target_directory: PathBuf,
    icon: Option<PathBuf>,
    options: BundleOptions,
}

impl Package {
    fn resolve(metadata: &Value, name: &str, platform: Platform) -> Result<Self, String> {
        let packages = metadata["packages"]
            .as_array()
            .ok_or_else(|| "cargo metadata output has no package list".to_owned())?;
        let matches = packages
            .iter()
            .filter(|package| package["source"].is_null() && package["name"].as_str() == Some(name))
            .collect::<Vec<_>>();
        let package = match matches.as_slice() {
            [package] => *package,
            [] => return Err(format!("ice bundle: package `{name}` was not found")),
            _ => return Err(format!("ice bundle: package `{name}` is ambiguous")),
        };
        let manifest = package["manifest_path"]
            .as_str()
            .ok_or_else(|| format!("ice bundle: package `{name}` has no manifest path"))?;
        let root = Path::new(manifest)
            .parent()
            .ok_or_else(|| format!("ice bundle: package `{name}` has no package directory"))?
            .to_path_buf();
        let options = BundleOptions::read(&package["metadata"]["ice"]["bundle"])?;
        let (binary, source) = binary_target(package, name, options.executable.as_deref())?;
        let icon = options
            .icon
            .as_ref()
            .map(|icon| resolve_icon(&root, icon))
            .transpose()?;
        Ok(Self {
            root,
            name: name.to_owned(),
            version: package["version"]
                .as_str()
                .ok_or_else(|| format!("ice bundle: package `{name}` has no version"))?
                .to_owned(),
            description: text(&package["description"]),
            authors: package["authors"]
                .as_array()
                .map(Vec::as_slice)
                .unwrap_or_default()
                .iter()
                .filter_map(|author| author.as_str().map(str::to_owned))
                .collect(),
            homepage: text(&package["homepage"]),
            executable: format!("{binary}{}", platform.executable_suffix()),
            source,
            target_directory: metadata["target_directory"]
                .as_str()
                .ok_or_else(|| "cargo metadata output has no target directory".to_owned())?
                .into(),
            icon,
            options,
        })
    }
}

fn text(value: &Value) -> Option<String> {
    value
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

/// Finds the binary a bundle wraps, along with the crate root it is built
/// from, which is where a Windows subsystem attribute would have to live.
fn binary_target(
    package: &Value,
    name: &str,
    requested: Option<&str>,
) -> Result<(String, PathBuf), String> {
    let binaries = package["targets"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter()
        .filter(|target| {
            target["kind"]
                .as_array()
                .is_some_and(|kinds| kinds.iter().any(|kind| kind == "bin"))
        })
        .filter_map(|target| Some((target["name"].as_str()?, target["src_path"].as_str()?)))
        .collect::<Vec<_>>();
    let selected = match requested {
        Some(requested) => binaries
            .iter()
            .find(|(binary, _)| *binary == requested)
            .ok_or_else(|| {
                format!("ice bundle: package `{name}` has no binary target `{requested}`")
            })?,
        None => match binaries.as_slice() {
            [binary] => binary,
            [] => return Err(format!("ice bundle: package `{name}` has no binary target")),
            _ => {
                return Err(format!(
                    "ice bundle: package `{name}` has several binary targets ({}); name one under [package.metadata.ice.bundle] executable",
                    binaries
                        .iter()
                        .map(|(binary, _)| *binary)
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
        },
    };
    Ok((selected.0.to_owned(), PathBuf::from(selected.1)))
}

fn resolve_icon(root: &Path, icon: &str) -> Result<PathBuf, String> {
    let path = root.join(icon);
    if path.extension().and_then(|extension| extension.to_str()) != Some("svg") {
        return Err(format!(
            "bundle icon `{icon}` must be an .svg; every raster size is rendered from it"
        ));
    }
    if !path.is_file() {
        return Err(format!("bundle icon `{}` does not exist", path.display()));
    }
    Ok(path)
}

/// The parts of a bundle no standard Cargo or Ice declaration can express.
#[derive(Debug, Default, PartialEq, Eq)]
struct BundleOptions {
    name: Option<String>,
    identifier: Option<String>,
    executable: Option<String>,
    icon: Option<String>,
    category: Option<String>,
    copyright: Option<String>,
    minimum_system_version: Option<String>,
    usage: BTreeMap<String, String>,
}

impl BundleOptions {
    fn read(table: &Value) -> Result<Self, String> {
        if table.is_null() {
            return Ok(Self::default());
        }
        let table = table
            .as_object()
            .ok_or_else(|| "[package.metadata.ice.bundle] must be a table".to_owned())?;
        let known = [
            "name",
            "identifier",
            "executable",
            "icon",
            "category",
            "copyright",
            "minimum-system-version",
            "usage",
        ];
        if let Some(unknown) = table.keys().find(|key| !known.contains(&key.as_str())) {
            return Err(format!(
                "[package.metadata.ice.bundle] has no `{unknown}` key; known keys are {}",
                known.join(", ")
            ));
        }
        let string = |key: &str| -> Result<Option<String>, String> {
            match table.get(key) {
                None => Ok(None),
                Some(Value::String(value)) if !value.is_empty() => Ok(Some(value.clone())),
                Some(_) => Err(format!(
                    "[package.metadata.ice.bundle] `{key}` must be a non-empty string"
                )),
            }
        };
        Ok(Self {
            name: string("name")?,
            identifier: string("identifier")?,
            executable: string("executable")?,
            icon: string("icon")?,
            category: string("category")?,
            copyright: string("copyright")?,
            minimum_system_version: string("minimum-system-version")?,
            usage: usage_descriptions(table.get("usage").unwrap_or(&Value::Null))?,
        })
    }
}

/// Reads `[package.metadata.ice.bundle.usage]` into the Info.plist keys macOS
/// reads, ordered so two builds of one manifest write the same file.
fn usage_descriptions(table: &Value) -> Result<BTreeMap<String, String>, String> {
    if table.is_null() {
        return Ok(BTreeMap::new());
    }
    let table = table
        .as_object()
        .ok_or_else(|| "[package.metadata.ice.bundle.usage] must be a table".to_owned())?;
    let mut descriptions = BTreeMap::new();
    for (declared, value) in table {
        let key = usage_key(declared)?;
        let reason = value
            .as_str()
            .map(str::trim)
            .filter(|reason| !reason.is_empty())
            .ok_or_else(|| {
                format!(
                    "[package.metadata.ice.bundle.usage] `{declared}` must be a non-empty string; macOS shows it verbatim in the permission prompt"
                )
            })?;
        if descriptions
            .insert(key.clone(), reason.to_owned())
            .is_some()
        {
            return Err(format!(
                "[package.metadata.ice.bundle.usage] declares `{key}` twice, once through its shorthand"
            ));
        }
    }
    Ok(descriptions)
}

/// A shorthand names one of the two common permissions; anything else is the
/// Info.plist key itself, so a typo is refused instead of reaching a bundle
/// that crashes on the permission it meant to describe.
fn usage_key(declared: &str) -> Result<String, String> {
    if let Some((_, key)) = USAGE_ALIASES.iter().find(|(alias, _)| *alias == declared) {
        return Ok((*key).to_owned());
    }
    let named_permission = declared.len() > USAGE_KEY_PREFIX.len() + USAGE_KEY_SUFFIX.len();
    let is_plist_key = declared.starts_with(USAGE_KEY_PREFIX)
        && declared.ends_with(USAGE_KEY_SUFFIX)
        && named_permission;
    if is_plist_key {
        return Ok(declared.to_owned());
    }
    let aliases = USAGE_ALIASES
        .iter()
        .map(|(alias, _)| *alias)
        .collect::<Vec<_>>()
        .join(", ");
    Err(format!(
        "[package.metadata.ice.bundle.usage] `{declared}` is neither a shorthand ({aliases}) nor an `{USAGE_KEY_PREFIX}…{USAGE_KEY_SUFFIX}` key"
    ))
}

/// One resolved identity, complete for the host platform before any file is
/// written, so a missing field is a message rather than a broken package.
#[derive(Debug, PartialEq, Eq)]
struct BundleMeta {
    name: String,
    package: String,
    identifier: String,
    executable: String,
    version: String,
    description: String,
    maintainer: String,
    homepage: Option<String>,
    icon: bool,
    category: Option<String>,
    copyright: Option<String>,
    minimum_system_version: String,
    usage: BTreeMap<String, String>,
}

impl BundleMeta {
    /// Reads the app declaration through the ordinary analysis path, so a
    /// graph that does not check never reaches a release build or a signature.
    fn resolve(package: &Package, platform: Platform) -> Result<Self, String> {
        let source = ice_root(&package.root)?;
        let analysis = ui_lang_core::AnalysisDb::default()
            .analyze_root(&source)
            .map_err(|error| error.render(&source.display().to_string()))?;
        let document = analysis.document.source_document();
        let identifier = package
            .options
            .identifier
            .clone()
            .or_else(|| document.settings.id.clone())
            .ok_or_else(|| {
                format!(
                    "`{}` declares no app `id`; a bundle identifier is required, so set one there or under [package.metadata.ice.bundle]",
                    source.display()
                )
            })?;
        let name = package
            .options
            .name
            .clone()
            .unwrap_or_else(|| document.app.clone());
        let description = package
            .description
            .clone()
            .ok_or_else(|| manifest_field(&package.name, "description"))?;
        let maintainer = package
            .authors
            .first()
            .cloned()
            .ok_or_else(|| manifest_field(&package.name, "authors"))?;
        if platform == Platform::Windows {
            check_windows_subsystem(&package.source)?;
        }
        if platform == Platform::Linux {
            check_debian_name(&package.name)?;
        }
        Ok(Self {
            name,
            package: package.name.clone(),
            identifier,
            executable: package.executable.clone(),
            version: package.version.clone(),
            description,
            maintainer,
            homepage: package.homepage.clone(),
            icon: package.icon.is_some(),
            category: package.options.category.clone(),
            copyright: package.options.copyright.clone(),
            minimum_system_version: package
                .options
                .minimum_system_version
                .clone()
                .unwrap_or_else(|| DEFAULT_MINIMUM_SYSTEM_VERSION.to_owned()),
            usage: package.options.usage.clone(),
        })
    }
}

fn manifest_field(package: &str, field: &str) -> String {
    format!("package `{package}` sets no `{field}`; an installable package has to name one")
}

/// A Rust binary is a console program unless its crate root says otherwise, so
/// an installed release would open a terminal behind its window. The attribute
/// is crate-level, which no generated code can add, and the console is
/// invisible until someone runs the installed build — so it is checked here.
fn check_windows_subsystem(source: &Path) -> Result<(), String> {
    if read_to_string(source)?.contains("windows_subsystem") {
        return Ok(());
    }
    Err(format!(
        "`{}` does not set `windows_subsystem`, so the installed application would open a console window behind it; add `#![cfg_attr(not(debug_assertions), windows_subsystem = \"windows\")]` above its first item",
        source.display()
    ))
}

/// dpkg accepts only these characters, and rejects the package at build time
/// rather than explaining which character it disliked.
fn check_debian_name(package: &str) -> Result<(), String> {
    let valid = package.len() >= 2
        && package.starts_with(|first: char| first.is_ascii_lowercase() || first.is_ascii_digit())
        && package.chars().all(|character| {
            character.is_ascii_lowercase()
                || character.is_ascii_digit()
                || "+-.".contains(character)
        });
    if valid {
        Ok(())
    } else {
        Err(format!(
            "`{package}` is not a Debian package name; it must start with a lower-case letter or digit and use only those, digits, and `+-.`"
        ))
    }
}

fn ice_root(package_root: &Path) -> Result<PathBuf, String> {
    let roots = crate::root_files(&crate::ice_files(package_root)?)
        .map_err(|_| "ice bundle: package has no Ice app or daemon root".to_owned())?;
    match roots.as_slice() {
        [root] => Ok(root.clone()),
        _ => Err(format!(
            "ice bundle: package has several Ice roots: {}",
            roots
                .iter()
                .map(|root| root.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )),
    }
}

/// Where Cargo left each requested build. Naming `--target` moves the profile
/// directory under the triple, and naming none leaves it at the top.
fn binary_paths(package: &Package, request: &Request) -> Vec<PathBuf> {
    let release = |directory: PathBuf| directory.join("release").join(&package.executable);
    match request.targets.as_slice() {
        [] => vec![release(package.target_directory.clone())],
        targets => targets
            .iter()
            .map(|target| release(package.target_directory.join(target)))
            .collect(),
    }
}

/// Links one executable for the bundle, joining several architectures into a
/// universal binary when the request named more than one target.
fn link_executable(package: &Package, request: &Request, output: &Path) -> Result<PathBuf, String> {
    let binaries = binary_paths(package, request);
    if let [binary] = binaries.as_slice() {
        return Ok(binary.clone());
    }
    let universal = output.join(format!("{}-universal", package.executable));
    let mut arguments = vec!["-create".to_owned(), "-output".to_owned(), path(&universal)];
    arguments.extend(binaries.iter().map(|binary| path(binary)));
    tool("lipo", &arguments)?;
    Ok(universal)
}

/// A workflow that maps an unset repository secret onto an environment
/// variable sets it to the empty string, which is the same as not having it.
fn setting(name: &str) -> Option<String> {
    env::var(name).ok().filter(|value| !value.is_empty())
}

fn path(value: &Path) -> String {
    value.display().to_string()
}

fn recreate(directory: &Path) -> Result<(), String> {
    if directory.exists() {
        fs::remove_dir_all(directory)
            .map_err(|error| format!("cannot clear `{}`: {error}", directory.display()))?;
    }
    create_dir(directory)
}

fn create_dir(directory: &Path) -> Result<(), String> {
    fs::create_dir_all(directory)
        .map_err(|error| format!("cannot create `{}`: {error}", directory.display()))
}

fn install(from: &Path, to: &Path) -> Result<(), String> {
    fs::copy(from, to)
        .map(|_| ())
        .map_err(|error| format!("cannot copy `{}` into the package: {error}", from.display()))
}

fn read(file: &Path) -> Result<Vec<u8>, String> {
    fs::read(file).map_err(|error| format!("cannot read `{}`: {error}", file.display()))
}

fn read_to_string(file: &Path) -> Result<String, String> {
    fs::read_to_string(file).map_err(|error| format!("cannot read `{}`: {error}", file.display()))
}

fn write(file: &Path, bytes: &[u8]) -> Result<(), String> {
    fs::write(file, bytes).map_err(|error| format!("cannot write `{}`: {error}", file.display()))
}

fn tool(program: &str, arguments: &[String]) -> Result<(), String> {
    let status = Command::new(program)
        .args(arguments)
        .status()
        .map_err(|error| format!("cannot run {program}: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{program} {} failed", arguments.join(" ")))
    }
}

fn capture(
    program: &str,
    arguments: &[String],
    directory: Option<&Path>,
) -> Result<String, String> {
    let mut command = Command::new(program);
    command.args(arguments);
    if let Some(directory) = directory {
        command.current_dir(directory);
    }
    let output = command
        .output()
        .map_err(|error| format!("cannot run {program}: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "{program} {} failed: {}",
            arguments.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    pub(crate) fn showcase_meta() -> BundleMeta {
        BundleMeta {
            name: "Showcase".into(),
            package: "showcase".into(),
            identifier: "dev.ducktape.ui.showcase".into(),
            executable: "showcase".into(),
            version: "0.1.0".into(),
            description: "Default iced components, composed and checked by Ice.".into(),
            maintainer: "ducktape-ui <noreply@example.invalid>".into(),
            homepage: Some("https://github.com/byeongsu-hong/ducktape-ui".into()),
            icon: true,
            category: Some("public.app-category.developer-tools".into()),
            copyright: None,
            minimum_system_version: "11.0".into(),
            usage: BTreeMap::new(),
        }
    }

    #[test]
    #[ignore = "allocation contract; run alone with --test-threads=1"]
    fn allocation_contract_bundle_identity_borrows_source_document() {
        const STATES: usize = 256;
        let fixture = tempfile::tempdir().expect("temporary bundle root");
        let mut source = String::from(
            "app Allocation\n  id \"dev.ducktape.allocation\"\n\
             theme contract AppTheme\n  bg\n  fg\n  primary\n  danger\n\
             palette app for AppTheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\n\
             state\n",
        );
        for index in 0..STATES {
            source.push_str(&format!("  value_{index} = {index}\n"));
        }
        source.push_str("view\n  text \"ready\"\n");
        fs::write(fixture.path().join("app.ice"), source).expect("write Ice root");
        let package = Package {
            root: fixture.path().to_owned(),
            name: "allocation".into(),
            version: "0.1.0".into(),
            description: Some("Allocation contract".into()),
            authors: vec!["Ice <ice@example.invalid>".into()],
            homepage: None,
            executable: "allocation".into(),
            source: fixture.path().join("main.rs"),
            target_directory: fixture.path().join("target"),
            icon: None,
            options: BundleOptions::default(),
        };

        // Blocks are the portable half of this measurement. The parser keeps a
        // span path per node, so the byte total scales with wherever
        // `tempdir()` landed: a `TMPDIR` 49 characters longer moved it from
        // 2,151,867 to 2,253,819 on one box while the block count did not
        // move at all. Borrowing the source document rather than copying it is
        // a statement about how many allocations resolving takes, so that is
        // what this pins; the bytes are reported, not asserted.
        const BLOCKS: u64 = 9_558;
        let mut resolved = None;
        let _profiler = dhat::Profiler::builder().testing().build();
        let measured = crate::allocation::clean_window(BLOCKS, || {
            resolved = Some(BundleMeta::resolve(&package, Platform::MacOs).expect("resolve"));
        });
        let meta = resolved.expect("resolve identity");

        assert_eq!(meta.name, "Allocation");
        assert_eq!(meta.identifier, "dev.ducktape.allocation");
        assert_eq!(
            measured.0, BLOCKS,
            "bundle identity allocations: {measured:?}"
        );
        eprintln!(
            "{STATES}-state bundle identity: {} heap blocks / {} bytes",
            measured.0, measured.1
        );
    }

    /// The per-platform packagers cannot run here, so this covers everything
    /// before them: a released app has to resolve to a complete, installable
    /// identity from its own manifest and Ice declaration, on any host.
    #[test]
    fn the_showcase_app_resolves_into_a_signable_bundle() {
        let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let metadata = crate::dev::metadata(&workspace).expect("read cargo metadata");
        let package =
            Package::resolve(&metadata, "showcase", Platform::MacOs).expect("resolve showcase");
        assert_eq!(package.executable, "showcase");
        assert!(
            package.icon.as_deref().is_some_and(Path::is_file),
            "showcase declares an icon that exists: {:?}",
            package.icon
        );

        let meta =
            BundleMeta::resolve(&package, Platform::MacOs).expect("resolve the bundle identity");
        assert_eq!(meta.name, "Showcase");
        assert_eq!(meta.identifier, "dev.ducktape.ui.showcase");
        assert_eq!(meta.package, "showcase");
        assert_eq!(meta.version, package.version);
        assert!(meta.icon);
        assert_eq!(
            meta.category.as_deref(),
            Some("public.app-category.developer-tools")
        );
        assert!(
            meta.copyright.is_some_and(|notice| notice.contains("MIT")),
            "a shipped bundle carries its licence notice"
        );
        // Every packager needs these, and only the Cargo manifest has them.
        assert!(!meta.description.is_empty(), "a description");
        assert!(meta.maintainer.contains('@'), "{}", meta.maintainer);
        assert!(meta.homepage.is_some());
    }

    /// Windows resolution is the same identity plus two platform demands, and
    /// both must hold for the app this repository actually publishes.
    #[test]
    fn the_showcase_app_resolves_for_every_platform() {
        let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let metadata = crate::dev::metadata(&workspace).expect("read cargo metadata");
        for (platform, executable) in [
            (Platform::Linux, "showcase"),
            (Platform::Windows, "showcase.exe"),
        ] {
            let package = Package::resolve(&metadata, "showcase", platform)
                .unwrap_or_else(|error| panic!("resolve showcase for {platform:?}: {error}"));
            assert_eq!(package.executable, executable);
            let meta = BundleMeta::resolve(&package, platform)
                .unwrap_or_else(|error| panic!("resolve {platform:?} identity: {error}"));
            assert_eq!(meta.executable, executable);
        }
    }

    #[test]
    fn a_console_window_behind_the_app_is_refused_on_windows() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let source = directory.path().join("main.rs");
        fs::write(&source, "fn main() {}\n").expect("write a crate root");
        let error = check_windows_subsystem(&source).expect_err("a console subsystem binary");
        assert!(error.contains("windows_subsystem"), "{error}");
        assert!(error.contains("cfg_attr"), "the message names the fix");

        fs::write(
            &source,
            "#![cfg_attr(not(debug_assertions), windows_subsystem = \"windows\")]\nfn main() {}\n",
        )
        .expect("write a windowed crate root");
        assert!(check_windows_subsystem(&source).is_ok());
    }

    #[test]
    fn debian_rejects_a_name_it_cannot_encode() {
        assert!(check_debian_name("showcase").is_ok());
        assert!(check_debian_name("ice-starter").is_ok());
        assert!(check_debian_name("Showcase").is_err(), "upper case");
        assert!(check_debian_name("my_app").is_err(), "underscore");
        assert!(check_debian_name("-app").is_err(), "leading dash");
        assert!(check_debian_name("a").is_err(), "one character");
    }

    #[test]
    fn a_named_target_moves_the_binary_under_its_triple() {
        let package = Package {
            root: "/workspace/examples/showcase".into(),
            name: "showcase".into(),
            version: "0.1.0".into(),
            description: None,
            authors: Vec::new(),
            homepage: None,
            executable: "showcase".into(),
            source: "/workspace/examples/showcase/src/main.rs".into(),
            target_directory: "/workspace/target".into(),
            icon: None,
            options: BundleOptions::default(),
        };
        assert_eq!(
            binary_paths(
                &package,
                &Request {
                    package: "showcase".into(),
                    targets: Vec::new()
                }
            ),
            [Path::new("/workspace/target/release/showcase")]
        );
        assert_eq!(
            binary_paths(
                &package,
                &Request {
                    package: "showcase".into(),
                    targets: vec!["aarch64-apple-darwin".into(), "x86_64-apple-darwin".into()],
                }
            ),
            [
                Path::new("/workspace/target/aarch64-apple-darwin/release/showcase"),
                Path::new("/workspace/target/x86_64-apple-darwin/release/showcase"),
            ]
        );
    }

    #[test]
    fn bundle_options_read_the_manifest_table() {
        let options = BundleOptions::read(&json!({
            "name": "Showcase",
            "icon": "../../assets/icons/ice.svg",
            "minimum-system-version": "12.0",
        }))
        .expect("read the table");
        assert_eq!(
            options,
            BundleOptions {
                name: Some("Showcase".into()),
                icon: Some("../../assets/icons/ice.svg".into()),
                minimum_system_version: Some("12.0".into()),
                ..BundleOptions::default()
            }
        );
        assert_eq!(
            BundleOptions::read(&Value::Null).expect("an absent table"),
            BundleOptions::default()
        );
    }

    #[test]
    fn the_usage_table_becomes_the_keys_macos_reads() {
        let options = BundleOptions::read(&json!({
            "usage": {
                "camera": "Ducktape uses the camera for video in huddles.",
                "microphone": "Ducktape uses the microphone for voice in huddles.",
                "NSSpeechRecognitionUsageDescription": "Ducktape transcribes what you say.",
            },
        }))
        .expect("read the usage table");
        assert_eq!(
            options.usage.into_iter().collect::<Vec<_>>(),
            [
                (
                    "NSCameraUsageDescription".to_owned(),
                    "Ducktape uses the camera for video in huddles.".to_owned()
                ),
                (
                    "NSMicrophoneUsageDescription".to_owned(),
                    "Ducktape uses the microphone for voice in huddles.".to_owned()
                ),
                (
                    "NSSpeechRecognitionUsageDescription".to_owned(),
                    "Ducktape transcribes what you say.".to_owned()
                ),
            ],
            "a shorthand and a written-out key land side by side, in a stable order"
        );
        assert!(
            BundleOptions::read(&json!({}))
                .expect("no usage table")
                .usage
                .is_empty()
        );
    }

    #[test]
    fn a_usage_key_macos_never_reads_is_refused() {
        let error = BundleOptions::read(&json!({ "usage": { "camara": "typo" } }))
            .expect_err("a misspelled shorthand");
        assert!(error.contains("`camara`"), "{error}");
        assert!(
            error.contains("camera, microphone"),
            "the message names both"
        );

        let error = BundleOptions::read(&json!({ "usage": { "NSCamera": "truncated" } }))
            .expect_err("a key that is not a usage description");
        assert!(error.contains("UsageDescription"), "{error}");
        assert!(
            BundleOptions::read(&json!({ "usage": { "NSUsageDescription": "empty name" } }))
                .is_err(),
            "a key that names no permission"
        );

        let error = BundleOptions::read(&json!({ "usage": { "camera": "  " } }))
            .expect_err("a blank reason is the crash it was meant to prevent");
        assert!(error.contains("non-empty string"), "{error}");
        assert!(
            BundleOptions::read(&json!({ "usage": { "camera": 7 } })).is_err(),
            "a reason is text"
        );
        assert!(
            BundleOptions::read(&json!({ "usage": "camera" })).is_err(),
            "the usage entry is a table"
        );
    }

    #[test]
    fn one_permission_cannot_be_declared_by_both_names() {
        let error = BundleOptions::read(&json!({
            "usage": {
                "camera": "one reason",
                "NSCameraUsageDescription": "another reason",
            },
        }))
        .expect_err("two reasons for one prompt");
        assert!(error.contains("NSCameraUsageDescription"), "{error}");
        assert!(error.contains("twice"), "{error}");
    }

    #[test]
    fn a_misspelled_bundle_key_is_reported_instead_of_ignored() {
        let error = BundleOptions::read(&json!({ "icons": "icon.svg" }))
            .expect_err("a misspelled key must not be silently dropped");
        assert!(error.contains("`icons`"), "{error}");
        let error = BundleOptions::read(&json!({ "name": 7 })).expect_err("a non-string name");
        assert!(error.contains("non-empty string"), "{error}");
    }

    #[test]
    fn an_svg_is_the_only_icon_source() {
        let directory = tempfile::tempdir().expect("temporary directory");
        fs::write(directory.path().join("icon.png"), b"not an svg").expect("write a raster icon");
        let error = resolve_icon(directory.path(), "icon.png").expect_err("a raster icon");
        assert!(error.contains(".svg"), "{error}");
        let error = resolve_icon(directory.path(), "missing.svg").expect_err("an absent icon");
        assert!(error.contains("does not exist"), "{error}");
    }

    #[test]
    fn requests_name_a_package_and_may_name_targets() {
        let parse = |args: &[&str]| {
            Request::parse(&args.iter().map(|arg| (*arg).to_owned()).collect::<Vec<_>>())
        };
        assert_eq!(
            parse(&["-p", "showcase"]).expect("a package request"),
            Request {
                package: "showcase".into(),
                targets: Vec::new(),
            }
        );
        assert_eq!(
            parse(&[
                "-p",
                "showcase",
                "--target",
                "aarch64-apple-darwin",
                "--target",
                "x86_64-apple-darwin",
            ])
            .expect("a universal request")
            .arch_label(),
            "universal"
        );
        assert_eq!(
            parse(&["-p", "showcase", "--target", "aarch64-apple-darwin"])
                .expect("one target")
                .arch_label(),
            "aarch64"
        );
        assert!(parse(&[]).is_err(), "a bundle needs a package");
        assert!(parse(&["-p"]).is_err(), "`-p` needs a value");
        assert!(parse(&["showcase"]).is_err(), "a bare package is not valid");
        assert!(
            parse(&[
                "-p",
                "showcase",
                "--target",
                "aarch64-apple-darwin",
                "--target",
                "aarch64-apple-darwin",
            ])
            .is_err(),
            "lipo cannot join a target with itself"
        );
    }

    #[test]
    fn the_build_is_locked_and_release() {
        assert_eq!(
            build_arguments(&Request {
                package: "showcase".into(),
                targets: vec!["aarch64-apple-darwin".into()],
            }),
            [
                "build",
                "--release",
                "--locked",
                "-p",
                "showcase",
                "--target",
                "aarch64-apple-darwin"
            ]
        );
    }
}
