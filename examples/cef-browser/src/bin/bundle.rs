use std::env;
use std::error::Error;
use std::fs;
use std::process::Command;

const APP: &str = "cef-browser-example";
#[cfg(target_os = "macos")]
const HELPER: &str = "cef-browser-helper";

fn main() -> Result<(), Box<dyn Error>> {
    let release = env::args().skip(1).any(|argument| argument == "--release");
    let metadata = cef::build_util::metadata::get_cargo_metadata()?;
    let profile = if release { "release" } else { "debug" };
    let target_path = metadata.target_directory().join(profile);
    let output = metadata.target_directory().join("cef-browser-bundle");
    fs::create_dir_all(&output)?;

    build(APP, release)?;
    #[cfg(target_os = "macos")]
    build(HELPER, release)?;

    #[cfg(target_os = "linux")]
    let executable = cef::build_util::linux::bundle(&output, &target_path, APP)?;

    #[cfg(target_os = "windows")]
    let executable = cef::build_util::win::bundle(&output, &target_path, APP)?;

    #[cfg(target_os = "macos")]
    let executable = {
        use cef::build_util::mac::{BundleInfo, bundle};

        let bundle_metadata = metadata.parse_bundle_metadata(APP)?;
        let info = BundleInfo::new(
            APP,
            "dev.ducktape.ice.cef-browser",
            "CEF in Ice",
            "English",
            semver::Version::new(0, 1, 0),
        );
        let app = bundle(
            &output,
            &target_path,
            APP,
            &bundle_metadata.helper_name,
            bundle_metadata.resources_path,
            info,
        )?;
        remove_credential_usage_descriptions(&app)?;
        app
    };

    println!("Run {}", executable.display());
    Ok(())
}

#[cfg(target_os = "macos")]
fn remove_credential_usage_descriptions(app: &std::path::Path) -> Result<(), Box<dyn Error>> {
    const PUBLIC_KEY_CREDENTIAL_USAGE: &str = "NSWebBrowserPublicKeyCredentialUsageDescription";

    fn scrub(plist_path: &std::path::Path) -> Result<(), Box<dyn Error>> {
        let mut value = plist::Value::from_file(plist_path)?;
        let dictionary = value
            .as_dictionary_mut()
            .ok_or_else(|| format!("{} is not a plist dictionary", plist_path.display()))?;
        if dictionary.remove(PUBLIC_KEY_CREDENTIAL_USAGE).is_none() {
            return Err(format!(
                "{} did not declare public-key credential access",
                plist_path.display()
            )
            .into());
        }
        value.to_file_xml(plist_path)?;
        Ok(())
    }

    scrub(&app.join("Contents/Info.plist"))?;
    for entry in fs::read_dir(app.join("Contents/Frameworks"))? {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type()?.is_dir()
            && path.extension().is_some_and(|extension| extension == "app")
        {
            scrub(&path.join("Contents/Info.plist"))?;
        }
    }
    Ok(())
}

fn build(binary: &str, release: bool) -> Result<(), Box<dyn Error>> {
    let mut command = Command::new(env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned()));
    command.args(["build", "-p", APP, "--features", "cef", "--bin", binary]);
    if release {
        command.arg("--release");
    }
    let status = command.status()?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("failed to build {binary}").into())
    }
}
