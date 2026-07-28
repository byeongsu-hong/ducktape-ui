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
        bundle(
            &output,
            &target_path,
            APP,
            &bundle_metadata.helper_name,
            bundle_metadata.resources_path,
            info,
        )?
    };

    println!("Run {}", executable.display());
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
