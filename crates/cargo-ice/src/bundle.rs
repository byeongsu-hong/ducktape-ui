//! macOS application bundling.
//!
//! `cargo ice bundle -p <package>` builds the package in release, lays a `.app`
//! out around the binary, renders the declared SVG icon into the `.icns` sizes
//! macOS asks for, code signs the bundle, and writes a `.dmg`. Notarization
//! runs when App Store Connect API credentials are present in the environment.
//!
//! The bundle identifier and display name come from the package's Ice `app`
//! declaration, which already names both; only the icon and the store metadata
//! macOS has no other source for live in `[package.metadata.ice.bundle]`.

use serde_json::Value;
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const USAGE: &str = "cargo ice bundle -p <package> [--target <triple>]...";
const DEFAULT_MINIMUM_SYSTEM_VERSION: &str = "11.0";
const AD_HOC_IDENTITY: &str = "-";

/// `.icns` entry types paired with the pixel size each carries. macOS chooses
/// between the 1x and 2x members of a pair by display scale, so 256 and 512 are
/// rendered once and stored under both of their type codes.
const ICNS_ENTRIES: &[(&str, u32)] = &[
    ("ic11", 32),
    ("ic12", 64),
    ("ic07", 128),
    ("ic08", 256),
    ("ic13", 256),
    ("ic09", 512),
    ("ic14", 512),
    ("ic10", 1024),
];

pub(crate) fn run(root: &Path, args: &[String]) -> Result<(), String> {
    let request = Request::parse(args)?;
    if !cfg!(target_os = "macos") {
        return Err("cargo ice bundle writes macOS bundles and must run on macOS".into());
    }
    let package = Package::resolve(&crate::dev::metadata(root)?, &request.package)?;
    let meta = BundleMeta::resolve(&package)?;
    let identity = signing_identity();
    let notary = Notary::from_env();
    check_signing_plan(&identity, notary.is_some())?;

    let build = build_arguments(&request);
    crate::cargo(&build.iter().map(String::as_str).collect::<Vec<_>>())?;

    let output = package.target_directory.join("ice-bundle");
    let executable = link_executable(&package, &request, &output)?;
    let app = output.join(format!("{}.app", meta.name));
    write_app(&app, &meta, &executable, package.icon.as_deref())?;
    sign(&app, &identity)?;

    let dmg = output.join(format!(
        "{}-{}-{}.dmg",
        meta.name,
        meta.version,
        request.arch_label()
    ));
    write_dmg(&app, &meta, &dmg)?;
    sign(&dmg, &identity)?;
    match notary {
        Some(notary) => notary.submit(&dmg)?,
        None => println!(
            "signed with `{identity}`; set ICE_NOTARY_KEY, ICE_NOTARY_KEY_ID, and ICE_NOTARY_ISSUER to notarize"
        ),
    }

    println!("{}", app.display());
    println!("{}", dmg.display());
    Ok(())
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

    /// Names the slice of hardware the `.dmg` runs on, so two architectures
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
    version: String,
    executable: String,
    target_directory: PathBuf,
    icon: Option<PathBuf>,
    options: BundleOptions,
}

impl Package {
    fn resolve(metadata: &Value, name: &str) -> Result<Self, String> {
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
        let version = package["version"]
            .as_str()
            .ok_or_else(|| format!("ice bundle: package `{name}` has no version"))?
            .to_owned();
        let options = BundleOptions::read(&package["metadata"]["ice"]["bundle"])?;
        let executable = options
            .executable
            .clone()
            .map_or_else(|| single_binary(package, name), Ok)?;
        let target_directory = metadata["target_directory"]
            .as_str()
            .ok_or_else(|| "cargo metadata output has no target directory".to_owned())?
            .into();
        let icon = options
            .icon
            .as_ref()
            .map(|icon| resolve_icon(&root, icon))
            .transpose()?;
        Ok(Self {
            root,
            version,
            executable,
            target_directory,
            icon,
            options,
        })
    }
}

fn single_binary(package: &Value, name: &str) -> Result<String, String> {
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
        .filter_map(|target| target["name"].as_str())
        .collect::<Vec<_>>();
    match binaries.as_slice() {
        [binary] => Ok((*binary).to_owned()),
        [] => Err(format!("ice bundle: package `{name}` has no binary target")),
        _ => Err(format!(
            "ice bundle: package `{name}` has several binary targets ({}); name one under [package.metadata.ice.bundle] executable",
            binaries.join(", ")
        )),
    }
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

/// The parts of a bundle macOS cannot infer from the Ice app declaration.
#[derive(Debug, Default, PartialEq, Eq)]
struct BundleOptions {
    name: Option<String>,
    identifier: Option<String>,
    executable: Option<String>,
    icon: Option<String>,
    category: Option<String>,
    copyright: Option<String>,
    minimum_system_version: Option<String>,
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
        })
    }
}

/// Everything written into `Info.plist`, resolved from the Ice app declaration
/// and the package manifest before any file is touched.
#[derive(Debug, PartialEq, Eq)]
struct BundleMeta {
    name: String,
    identifier: String,
    executable: String,
    version: String,
    icon: bool,
    category: Option<String>,
    copyright: Option<String>,
    minimum_system_version: String,
}

impl BundleMeta {
    /// Reads the app declaration through the ordinary analysis path, so a
    /// graph that does not check never reaches a release build or a signature.
    fn resolve(package: &Package) -> Result<Self, String> {
        let source = ice_root(&package.root)?;
        let analysis = ui_lang_core::AnalysisDb::default()
            .analyze_root(&source)
            .map_err(|error| error.render(&source.display().to_string()))?;
        let document = analysis.document.source_document().clone();
        let identifier = package
            .options
            .identifier
            .clone()
            .or(document.settings.id)
            .ok_or_else(|| {
                format!(
                    "`{}` declares no app `id`; macOS needs a bundle identifier, so set one there or under [package.metadata.ice.bundle]",
                    source.display()
                )
            })?;
        Ok(Self {
            name: package.options.name.clone().unwrap_or(document.app),
            identifier,
            executable: package.executable.clone(),
            version: package.version.clone(),
            icon: package.icon.is_some(),
            category: package.options.category.clone(),
            copyright: package.options.copyright.clone(),
            minimum_system_version: package
                .options
                .minimum_system_version
                .clone()
                .unwrap_or_else(|| DEFAULT_MINIMUM_SYSTEM_VERSION.to_owned()),
        })
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

fn info_plist(meta: &BundleMeta) -> plist::Value {
    let mut dictionary = plist::Dictionary::new();
    let mut set = |key: &str, value: plist::Value| {
        dictionary.insert(key.to_owned(), value);
    };
    let text = |value: &str| plist::Value::String(value.to_owned());
    set("CFBundleInfoDictionaryVersion", text("6.0"));
    set("CFBundlePackageType", text("APPL"));
    set("CFBundleName", text(&meta.name));
    set("CFBundleDisplayName", text(&meta.name));
    set("CFBundleIdentifier", text(&meta.identifier));
    set("CFBundleExecutable", text(&meta.executable));
    set("CFBundleShortVersionString", text(&meta.version));
    set("CFBundleVersion", text(&meta.version));
    set("LSMinimumSystemVersion", text(&meta.minimum_system_version));
    set("NSHighResolutionCapable", plist::Value::Boolean(true));
    set(
        "CFBundleSupportedPlatforms",
        plist::Value::Array(vec![text("MacOSX")]),
    );
    if meta.icon {
        set("CFBundleIconFile", text(&meta.name));
    }
    if let Some(category) = &meta.category {
        set("LSApplicationCategoryType", text(category));
    }
    if let Some(copyright) = &meta.copyright {
        set("NSHumanReadableCopyright", text(copyright));
    }
    plist::Value::Dictionary(dictionary)
}

fn write_app(
    app: &Path,
    meta: &BundleMeta,
    executable: &Path,
    icon: Option<&Path>,
) -> Result<(), String> {
    // A stale bundle keeps files the new one does not list, and codesign seals
    // whatever it finds, so the layout starts empty every time.
    if app.exists() {
        fs::remove_dir_all(app)
            .map_err(|error| format!("cannot clear `{}`: {error}", app.display()))?;
    }
    let contents = app.join("Contents");
    let binaries = contents.join("MacOS");
    let resources = contents.join("Resources");
    for directory in [&binaries, &resources] {
        fs::create_dir_all(directory)
            .map_err(|error| format!("cannot create `{}`: {error}", directory.display()))?;
    }
    fs::copy(executable, binaries.join(&meta.executable)).map_err(|error| {
        format!(
            "cannot copy `{}` into the bundle: {error}",
            executable.display()
        )
    })?;
    if let Some(icon) = icon {
        let svg =
            fs::read(icon).map_err(|error| format!("cannot read `{}`: {error}", icon.display()))?;
        let icns = icns_from_svg(&svg)?;
        let path = resources.join(format!("{}.icns", meta.name));
        fs::write(&path, icns)
            .map_err(|error| format!("cannot write `{}`: {error}", path.display()))?;
    }
    let path = contents.join("Info.plist");
    plist::to_file_xml(&path, &info_plist(meta))
        .map_err(|error| format!("cannot write `{}`: {error}", path.display()))
}

fn icns_from_svg(svg: &[u8]) -> Result<Vec<u8>, String> {
    // The renderer is built without font support, which would drop a `<text>`
    // element instead of drawing it. Saying so beats shipping a hole.
    if svg.windows(5).any(|window| window == b"<text") {
        return Err(
            "a bundle icon is rendered without fonts; convert its <text> elements to paths".into(),
        );
    }
    let tree = resvg::usvg::Tree::from_data(svg, &resvg::usvg::Options::default())
        .map_err(|error| format!("cannot read the bundle icon: {error}"))?;
    let mut rendered = BTreeMap::new();
    for (_, size) in ICNS_ENTRIES {
        if !rendered.contains_key(size) {
            rendered.insert(*size, render_png(&tree, *size)?);
        }
    }
    let mut body = Vec::new();
    for (kind, size) in ICNS_ENTRIES {
        let png = &rendered[size];
        let length = u32::try_from(png.len() + 8)
            .map_err(|_| format!("the {size}x{size} icon is too large for an .icns entry"))?;
        body.extend_from_slice(kind.as_bytes());
        body.extend_from_slice(&length.to_be_bytes());
        body.extend_from_slice(png);
    }
    let total = u32::try_from(body.len() + 8)
        .map_err(|_| "the rendered icon is too large for an .icns file".to_owned())?;
    let mut icns = Vec::with_capacity(body.len() + 8);
    icns.extend_from_slice(b"icns");
    icns.extend_from_slice(&total.to_be_bytes());
    icns.extend_from_slice(&body);
    Ok(icns)
}

fn render_png(tree: &resvg::usvg::Tree, size: u32) -> Result<Vec<u8>, String> {
    let mut pixmap = tiny_skia::Pixmap::new(size, size)
        .ok_or_else(|| format!("cannot allocate a {size}x{size} icon"))?;
    let scale = size as f32 / tree.size().width();
    resvg::render(
        tree,
        tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    pixmap
        .encode_png()
        .map_err(|error| format!("cannot encode the {size}x{size} icon: {error}"))
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
    fs::create_dir_all(output)
        .map_err(|error| format!("cannot create `{}`: {error}", output.display()))?;
    let universal = output.join(&package.executable);
    let mut arguments = vec!["-create".to_owned(), "-output".to_owned(), path(&universal)];
    arguments.extend(binaries.iter().map(|binary| path(binary)));
    tool("lipo", &arguments)?;
    Ok(universal)
}

/// Notarization only ever accepts a Developer ID signature, and it reports the
/// mismatch after the upload and the wait. Refusing the combination up front
/// turns a round trip to Apple into an immediate message.
fn check_signing_plan(identity: &str, notarizing: bool) -> Result<(), String> {
    if notarizing && identity == AD_HOC_IDENTITY {
        return Err(
            "notarization credentials are set but ICE_CODESIGN_IDENTITY is not; Apple rejects an ad-hoc signature"
                .into(),
        );
    }
    Ok(())
}

fn signing_identity() -> String {
    setting("ICE_CODESIGN_IDENTITY").unwrap_or_else(|| AD_HOC_IDENTITY.to_owned())
}

/// A workflow that maps an unset repository secret onto an environment
/// variable sets it to the empty string, which is the same as not having it.
fn setting(name: &str) -> Option<String> {
    env::var(name).ok().filter(|value| !value.is_empty())
}

fn sign(target: &Path, identity: &str) -> Result<(), String> {
    tool("codesign", &sign_arguments(target, identity))
}

fn sign_arguments(target: &Path, identity: &str) -> Vec<String> {
    let mut arguments = vec![
        "--force".to_owned(),
        "--sign".to_owned(),
        identity.to_owned(),
    ];
    if identity != AD_HOC_IDENTITY {
        // The hardened runtime and a trusted timestamp are both preconditions
        // for notarization; neither is available to an ad-hoc signature.
        arguments.extend([
            "--timestamp".to_owned(),
            "--options".to_owned(),
            "runtime".to_owned(),
        ]);
    }
    arguments.push(path(target));
    arguments
}

fn write_dmg(app: &Path, meta: &BundleMeta, dmg: &Path) -> Result<(), String> {
    let staging = dmg.with_extension("staging");
    if staging.exists() {
        fs::remove_dir_all(&staging)
            .map_err(|error| format!("cannot clear `{}`: {error}", staging.display()))?;
    }
    fs::create_dir_all(&staging)
        .map_err(|error| format!("cannot create `{}`: {error}", staging.display()))?;
    // `ditto` keeps the bundle's symlinks, modes, and extended attributes, so
    // the copy carries the same signature the original was sealed with.
    tool(
        "ditto",
        &[path(app), path(&staging.join(format!("{}.app", meta.name)))],
    )?;
    tool(
        "ln",
        &[
            "-s".to_owned(),
            "/Applications".to_owned(),
            path(&staging.join("Applications")),
        ],
    )?;
    tool(
        "hdiutil",
        &[
            "create".to_owned(),
            "-volname".to_owned(),
            meta.name.clone(),
            "-srcfolder".to_owned(),
            path(&staging),
            "-ov".to_owned(),
            "-format".to_owned(),
            "UDZO".to_owned(),
            path(dmg),
        ],
    )
}

struct Notary {
    key: String,
    key_id: String,
    issuer: String,
}

impl Notary {
    fn from_env() -> Option<Self> {
        let (Some(key), Some(key_id), Some(issuer)) = (
            setting("ICE_NOTARY_KEY"),
            setting("ICE_NOTARY_KEY_ID"),
            setting("ICE_NOTARY_ISSUER"),
        ) else {
            return None;
        };
        Some(Self {
            key,
            key_id,
            issuer,
        })
    }

    fn submit(&self, dmg: &Path) -> Result<(), String> {
        tool(
            "xcrun",
            &[
                "notarytool".to_owned(),
                "submit".to_owned(),
                path(dmg),
                "--key".to_owned(),
                self.key.clone(),
                "--key-id".to_owned(),
                self.key_id.clone(),
                "--issuer".to_owned(),
                self.issuer.clone(),
                "--wait".to_owned(),
            ],
        )?;
        // Stapling puts the notarization ticket inside the disk image, so a
        // first launch on a machine with no network still passes Gatekeeper.
        tool(
            "xcrun",
            &["stapler".to_owned(), "staple".to_owned(), path(dmg)],
        )
    }
}

fn path(value: &Path) -> String {
    value.display().to_string()
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn meta() -> BundleMeta {
        BundleMeta {
            name: "Showcase".into(),
            identifier: "dev.ducktape.ui.showcase".into(),
            executable: "showcase".into(),
            version: "0.1.0".into(),
            icon: true,
            category: Some("public.app-category.developer-tools".into()),
            copyright: None,
            minimum_system_version: "11.0".into(),
        }
    }

    fn plist_of(meta: &BundleMeta) -> plist::Dictionary {
        match info_plist(meta) {
            plist::Value::Dictionary(dictionary) => dictionary,
            other => panic!("Info.plist is not a dictionary: {other:?}"),
        }
    }

    #[test]
    fn info_plist_carries_what_gatekeeper_reads() {
        let plist = plist_of(&meta());
        for (key, value) in [
            ("CFBundleIdentifier", "dev.ducktape.ui.showcase"),
            ("CFBundleExecutable", "showcase"),
            ("CFBundleName", "Showcase"),
            ("CFBundleShortVersionString", "0.1.0"),
            ("CFBundleVersion", "0.1.0"),
            ("CFBundlePackageType", "APPL"),
            ("LSMinimumSystemVersion", "11.0"),
            ("CFBundleIconFile", "Showcase"),
            (
                "LSApplicationCategoryType",
                "public.app-category.developer-tools",
            ),
        ] {
            assert_eq!(
                plist.get(key).and_then(plist::Value::as_string),
                Some(value),
                "Info.plist {key}"
            );
        }
        assert_eq!(
            plist
                .get("NSHighResolutionCapable")
                .and_then(plist::Value::as_boolean),
            Some(true)
        );
        assert!(plist.get("NSHumanReadableCopyright").is_none());
    }

    #[test]
    fn an_icon_free_bundle_names_no_icon_file() {
        let plist = plist_of(&BundleMeta {
            icon: false,
            ..meta()
        });
        assert!(plist.get("CFBundleIconFile").is_none());
    }

    #[test]
    fn icns_stores_one_png_per_declared_size() {
        let svg =
            fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/icons/ice.svg"))
                .expect("read the repository icon");
        let icns = icns_from_svg(&svg).expect("render the icon");

        assert_eq!(&icns[..4], b"icns");
        assert_eq!(
            u32::from_be_bytes(icns[4..8].try_into().expect("length field")) as usize,
            icns.len(),
            "the header length must cover the whole file"
        );

        let mut offset = 8;
        let mut seen = Vec::new();
        while offset < icns.len() {
            let kind = std::str::from_utf8(&icns[offset..offset + 4]).expect("entry type");
            let length = u32::from_be_bytes(
                icns[offset + 4..offset + 8]
                    .try_into()
                    .expect("entry length"),
            ) as usize;
            let png = &icns[offset + 8..offset + length];
            assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n", "{kind} is not a PNG");
            let width = u32::from_be_bytes(png[16..20].try_into().expect("PNG width"));
            let height = u32::from_be_bytes(png[20..24].try_into().expect("PNG height"));
            seen.push((kind.to_owned(), width));
            assert_eq!(width, height, "{kind} is not square");
            offset += length;
        }
        assert_eq!(offset, icns.len(), "entries must tile the file exactly");
        assert_eq!(
            seen,
            ICNS_ENTRIES
                .iter()
                .map(|(kind, size)| ((*kind).to_owned(), *size))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn the_app_layout_places_the_three_files_macos_opens() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let executable = directory.path().join("showcase");
        fs::write(&executable, b"binary").expect("write the executable");
        let icon = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/icons/ice.svg");
        let app = directory.path().join("Showcase.app");
        fs::create_dir_all(app.join("Contents/MacOS")).expect("seed a stale bundle");
        fs::write(app.join("Contents/MacOS/stale"), b"old").expect("seed a stale file");

        write_app(&app, &meta(), &executable, Some(&icon)).expect("write the bundle");

        assert_eq!(
            fs::read(app.join("Contents/MacOS/showcase")).expect("bundled executable"),
            b"binary"
        );
        assert!(app.join("Contents/Resources/Showcase.icns").is_file());
        assert!(
            !app.join("Contents/MacOS/stale").exists(),
            "a rebuilt bundle must not keep files codesign would seal"
        );
        let plist = fs::read_to_string(app.join("Contents/Info.plist")).expect("Info.plist");
        assert!(plist.contains("<key>CFBundleIdentifier</key>"));
        assert!(plist.contains("<string>dev.ducktape.ui.showcase</string>"));
    }

    /// The macOS half of `run` cannot execute here, so this covers everything
    /// before it: a released app has to resolve to a complete, signable
    /// identity from its own manifest and Ice declaration, on any host.
    #[test]
    fn the_showcase_app_resolves_into_a_signable_bundle() {
        let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let metadata = crate::dev::metadata(&workspace).expect("read cargo metadata");
        let package = Package::resolve(&metadata, "showcase").expect("resolve showcase");
        assert_eq!(package.executable, "showcase");
        assert!(
            package.icon.as_deref().is_some_and(Path::is_file),
            "showcase declares an icon that exists: {:?}",
            package.icon
        );

        let meta = BundleMeta::resolve(&package).expect("resolve the bundle identity");
        assert_eq!(
            meta,
            BundleMeta {
                name: "Showcase".into(),
                identifier: "dev.ducktape.ui.showcase".into(),
                executable: "showcase".into(),
                version: package.version.clone(),
                icon: true,
                category: Some("public.app-category.developer-tools".into()),
                copyright: meta.copyright.clone(),
                minimum_system_version: DEFAULT_MINIMUM_SYSTEM_VERSION.into(),
            }
        );
        assert!(
            meta.copyright.is_some_and(|notice| notice.contains("MIT")),
            "a shipped bundle carries its licence notice"
        );
    }

    #[test]
    fn a_named_target_moves_the_binary_under_its_triple() {
        let package = Package {
            root: "/workspace/examples/showcase".into(),
            version: "0.1.0".into(),
            executable: "showcase".into(),
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

    /// Everything above this point runs on any host. This is the one check
    /// that drives the real `codesign`, `ditto`, and `hdiutil` sequence, so
    /// the tool arguments are wrong here rather than on a release tag.
    #[cfg(target_os = "macos")]
    #[test]
    fn a_signed_bundle_becomes_a_mountable_disk_image() {
        let directory = tempfile::tempdir().expect("temporary directory");
        // The test binary is a real Mach-O for this architecture, which is
        // what codesign needs; no system path is assumed to be signable.
        let executable = directory.path().join("showcase");
        fs::copy(env::current_exe().expect("this test binary"), &executable)
            .expect("stage an executable");
        let meta = meta();
        let app = directory.path().join("Showcase.app");
        let icon = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/icons/ice.svg");

        write_app(&app, &meta, &executable, Some(&icon)).expect("write the bundle");
        sign(&app, AD_HOC_IDENTITY).expect("sign the bundle");
        tool(
            "codesign",
            &["--verify".to_owned(), "--strict".to_owned(), path(&app)],
        )
        .expect("the bundle signature verifies");

        let dmg = directory.path().join("Showcase-0.1.0-test.dmg");
        write_dmg(&app, &meta, &dmg).expect("write the disk image");
        tool("hdiutil", &["verify".to_owned(), path(&dmg)]).expect("the disk image verifies");
        assert!(dmg.is_file(), "the disk image is where the release looks");
    }

    #[test]
    fn an_icon_that_needs_a_font_is_refused() {
        let lettered = br##"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16">
            <text x="2" y="12">I</text></svg>"##;
        let error = icns_from_svg(lettered).expect_err("a text icon cannot render");
        assert!(error.contains("paths"), "{error}");
        let drawn = br##"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16">
            <rect width="16" height="16" fill="#000"/></svg>"##;
        assert!(icns_from_svg(drawn).is_ok(), "a drawn icon still renders");
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

    #[test]
    fn notarizing_without_an_identity_fails_before_the_upload() {
        let identity = "Developer ID Application: Example (TEAMID)";
        assert!(check_signing_plan(identity, true).is_ok());
        assert!(check_signing_plan(identity, false).is_ok());
        assert!(check_signing_plan(AD_HOC_IDENTITY, false).is_ok());
        let error = check_signing_plan(AD_HOC_IDENTITY, true)
            .expect_err("an ad-hoc signature cannot be notarized");
        assert!(error.contains("ICE_CODESIGN_IDENTITY"), "{error}");
    }

    #[test]
    fn a_real_identity_signs_for_the_hardened_runtime() {
        // codesign refuses `--options runtime` alongside an ad-hoc signature,
        // and notarization refuses a bundle that was signed without it.
        let app = Path::new("/tmp/Showcase.app");
        let ad_hoc = sign_arguments(app, AD_HOC_IDENTITY);
        assert_eq!(
            ad_hoc,
            ["--force", "--sign", "-", "/tmp/Showcase.app"],
            "an ad-hoc signature takes no runtime or timestamp options"
        );
        assert_eq!(
            sign_arguments(app, "Developer ID Application: Example (TEAMID)"),
            [
                "--force",
                "--sign",
                "Developer ID Application: Example (TEAMID)",
                "--timestamp",
                "--options",
                "runtime",
                "/tmp/Showcase.app",
            ]
        );
    }
}
