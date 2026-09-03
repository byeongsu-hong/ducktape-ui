//! macOS application bundling: `.app`, code signature, `.dmg`, notarization.

use super::{BundleMeta, icon, path, setting, tool};
use std::fs;
use std::path::{Path, PathBuf};

const AD_HOC_IDENTITY: &str = "-";

pub(super) fn bundle(
    output: &Path,
    meta: &BundleMeta,
    executable: &Path,
    icon: Option<&Path>,
    arch: &str,
) -> Result<Vec<PathBuf>, String> {
    let identity = signing_identity();
    let notary = Notary::from_env();
    check_signing_plan(&identity, notary.is_some())?;

    let app = output.join(format!("{}.app", meta.name));
    write_app(&app, meta, executable, icon)?;
    sign(&app, &identity)?;

    let dmg = output.join(format!("{}-{}-{arch}.dmg", meta.name, meta.version));
    write_dmg(&app, meta, &dmg)?;
    sign(&dmg, &identity)?;
    match notary {
        Some(notary) => notary.submit(&dmg)?,
        None => println!(
            "signed with `{identity}`; set ICE_NOTARY_KEY, ICE_NOTARY_KEY_ID, and ICE_NOTARY_ISSUER to notarize"
        ),
    }
    Ok(vec![app, dmg])
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
    // Without the reason for a protected resource, the first call that touches
    // it terminates the process; there is no prompt and no recoverable error.
    for (key, reason) in &meta.usage {
        set(key, text(reason));
    }
    plist::Value::Dictionary(dictionary)
}

fn write_app(
    app: &Path,
    meta: &BundleMeta,
    executable: &Path,
    source: Option<&Path>,
) -> Result<(), String> {
    // A stale bundle keeps files the new one does not list, and codesign seals
    // whatever it finds, so the layout starts empty every time.
    super::recreate(app)?;
    let contents = app.join("Contents");
    let binaries = contents.join("MacOS");
    let resources = contents.join("Resources");
    for directory in [&binaries, &resources] {
        fs::create_dir_all(directory)
            .map_err(|error| format!("cannot create `{}`: {error}", directory.display()))?;
    }
    super::install(executable, &binaries.join(&meta.executable))?;
    if let Some(source) = source {
        let svg = super::read(source)?;
        super::write(
            &resources.join(format!("{}.icns", meta.name)),
            &icon::icns(&svg)?,
        )?;
    }
    let plist = contents.join("Info.plist");
    plist::to_file_xml(&plist, &info_plist(meta))
        .map_err(|error| format!("cannot write `{}`: {error}", plist.display()))
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
    super::recreate(&staging)?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bundle::tests::showcase_meta;

    fn plist_of(meta: &BundleMeta) -> plist::Dictionary {
        match info_plist(meta) {
            plist::Value::Dictionary(dictionary) => dictionary,
            other => panic!("Info.plist is not a dictionary: {other:?}"),
        }
    }

    #[test]
    fn info_plist_carries_what_gatekeeper_reads() {
        let plist = plist_of(&showcase_meta());
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
    fn a_declared_permission_reaches_the_prompt_macos_shows() {
        let plist = plist_of(&BundleMeta {
            usage: [
                (
                    "NSCameraUsageDescription".to_owned(),
                    "Ducktape uses the camera for video in huddles.".to_owned(),
                ),
                (
                    "NSMicrophoneUsageDescription".to_owned(),
                    "Ducktape uses the microphone for voice in huddles.".to_owned(),
                ),
            ]
            .into_iter()
            .collect(),
            ..showcase_meta()
        });
        assert_eq!(
            plist
                .get("NSCameraUsageDescription")
                .and_then(plist::Value::as_string),
            Some("Ducktape uses the camera for video in huddles.")
        );
        assert_eq!(
            plist
                .get("NSMicrophoneUsageDescription")
                .and_then(plist::Value::as_string),
            Some("Ducktape uses the microphone for voice in huddles.")
        );
        assert!(
            plist_of(&showcase_meta())
                .get("NSCameraUsageDescription")
                .is_none(),
            "an app that declares nothing asks for nothing"
        );
    }

    #[test]
    fn an_icon_free_bundle_names_no_icon_file() {
        let plist = plist_of(&BundleMeta {
            icon: false,
            ..showcase_meta()
        });
        assert!(plist.get("CFBundleIconFile").is_none());
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
        assert_eq!(
            sign_arguments(app, AD_HOC_IDENTITY),
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

    #[test]
    fn the_app_layout_places_the_three_files_macos_opens() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let executable = directory.path().join("showcase");
        fs::write(&executable, b"binary").expect("write the executable");
        let icon = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/icons/ice.svg");
        let app = directory.path().join("Showcase.app");
        fs::create_dir_all(app.join("Contents/MacOS")).expect("seed a stale bundle");
        fs::write(app.join("Contents/MacOS/stale"), b"old").expect("seed a stale file");

        write_app(&app, &showcase_meta(), &executable, Some(&icon)).expect("write the bundle");

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

    /// The one check that drives the real `codesign`, `ditto`, and `hdiutil`
    /// sequence, so the tool arguments are wrong here rather than on a tag.
    #[cfg(target_os = "macos")]
    #[test]
    fn a_signed_bundle_becomes_a_disk_image() {
        /// Every UDIF image ends with a 512-byte trailer, and these four bytes
        /// are what identifies one.
        const UDIF_TRAILER_SIZE: usize = 512;
        const UDIF_MAGIC: &[u8] = b"koly";

        let directory = tempfile::tempdir().expect("temporary directory");
        // The test binary is a real Mach-O for this architecture, which is
        // what codesign needs; no system path is assumed to be signable.
        let executable = directory.path().join("showcase");
        fs::copy(
            std::env::current_exe().expect("this test binary"),
            &executable,
        )
        .expect("stage an executable");
        let meta = showcase_meta();
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
        // `hdiutil verify` attaches the image through DiskArbitration, which
        // answered `Resource temporarily unavailable` on a loaded runner and
        // left an orphaned helper behind — a coin flip against every pull
        // request. `hdiutil create` already reported success; what is left to
        // check is that it wrote a UDIF image, and the trailer says so without
        // asking the kernel for a device.
        let image = fs::read(&dmg).expect("read the disk image");
        let trailer = image
            .len()
            .checked_sub(UDIF_TRAILER_SIZE)
            .expect("a disk image ends with a 512-byte trailer");
        assert_eq!(
            &image[trailer..trailer + 4],
            UDIF_MAGIC,
            "`{}` is not a UDIF disk image",
            dmg.display()
        );
    }
}
