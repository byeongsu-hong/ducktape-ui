//! Windows packaging: a per-user MSI with a Start menu shortcut, authored for
//! the WiX toolset and signed when a certificate is available.

use super::{BundleMeta, icon, path, setting, tool};
use sha2::{Digest, Sha256};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

const DEFAULT_TIMESTAMP_URL: &str = "http://timestamp.digicert.com";

pub(super) fn bundle(
    output: &Path,
    meta: &BundleMeta,
    executable: &Path,
    source: Option<&Path>,
    arch: &str,
) -> Result<Vec<PathBuf>, String> {
    let staged = output.join(&meta.executable);
    super::install(executable, &staged)?;
    // The executable is signed before it is carried into the package, so the
    // file a user runs is signed whether they install or copy it out.
    sign(&staged)?;

    let icon_file = output.join(format!("{}.ico", meta.name));
    if let Some(source) = source {
        super::write(&icon_file, &icon::ico(&super::read(source)?)?)?;
    }

    let authoring = output.join(format!("{}.wxs", meta.name));
    super::write(
        &authoring,
        package_authoring(meta, &staged, source.map(|_| icon_file.as_path()))?.as_bytes(),
    )?;

    let msi = output.join(format!("{}-{}-{arch}.msi", meta.name, meta.version));
    tool(
        "wix",
        &[
            "build".to_owned(),
            "-arch".to_owned(),
            wix_architecture(arch)?.to_owned(),
            path(&authoring),
            "-o".to_owned(),
            path(&msi),
        ],
    )?;
    sign(&msi)?;
    Ok(vec![msi])
}

fn wix_architecture(arch: &str) -> Result<&'static str, String> {
    match arch {
        "x86_64" => Ok("x64"),
        "aarch64" => Ok("arm64"),
        other => Err(format!("no Windows installer architecture for `{other}`")),
    }
}

/// A per-user package: it installs under the user's local application data
/// and writes one Start menu shortcut, so no elevation prompt stands between
/// a download and a running application.
fn package_authoring(
    meta: &BundleMeta,
    executable: &Path,
    icon_file: Option<&Path>,
) -> Result<String, String> {
    let icon_authoring = icon_file.map_or_else(String::new, |icon| {
        format!(
            "    <Icon Id=\"AppIcon\" SourceFile=\"{}\" />\n\
             \x20   <Property Id=\"ARPPRODUCTICON\" Value=\"AppIcon\" />\n",
            escape(&path(icon))
        )
    });
    let shortcut_icon = if icon_file.is_some() {
        " Icon=\"AppIcon\""
    } else {
        ""
    };
    Ok(format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<Wix xmlns="http://wixtoolset.org/schemas/v4/wxs">
  <Package Name="{name}" Manufacturer="{manufacturer}" Version="{version}"
           UpgradeCode="{upgrade_code}" Scope="perUser" Compressed="yes">
    <MajorUpgrade DowngradeErrorMessage="A newer version of [ProductName] is already installed." />
    <MediaTemplate EmbedCab="yes" />
{icon_authoring}    <StandardDirectory Id="LocalAppDataFolder">
      <Directory Id="INSTALLFOLDER" Name="{name}" />
    </StandardDirectory>
    <StandardDirectory Id="ProgramMenuFolder" />
    <Feature Id="Main">
      <Component Directory="INSTALLFOLDER">
        <File Id="AppExecutable" Source="{executable}" Name="{executable_name}" KeyPath="yes" />
      </Component>
      <Component Directory="ProgramMenuFolder">
        <Shortcut Id="AppShortcut" Name="{name}" Target="[INSTALLFOLDER]{executable_name}"
                  WorkingDirectory="INSTALLFOLDER"{shortcut_icon} />
        <RegistryValue Root="HKCU" Key="Software\{identifier}" Name="installed"
                       Type="integer" Value="1" KeyPath="yes" />
      </Component>
    </Feature>
  </Package>
</Wix>
"#,
        name = escape(&meta.name),
        manufacturer = escape(manufacturer(&meta.maintainer)),
        version = product_version(&meta.version)?,
        upgrade_code = upgrade_code(&meta.identifier),
        executable = escape(&path(executable)),
        executable_name = escape(&meta.executable),
        identifier = escape(&meta.identifier),
    ))
}

/// `Name <address>` is the Debian maintainer shape; Windows wants the name.
fn manufacturer(maintainer: &str) -> &str {
    maintainer
        .split('<')
        .next()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .unwrap_or(maintainer)
}

/// Windows Installer compares three numeric fields, with a one-byte major and
/// minor. A version it cannot represent has to be reported, because a silently
/// truncated one would make an upgrade look like a downgrade.
fn product_version(version: &str) -> Result<String, String> {
    let release = version
        .split(['-', '+'])
        .next()
        .expect("a split always yields one field");
    let fields = release.split('.').collect::<Vec<_>>();
    let mut numbers = Vec::new();
    for (index, field) in fields.iter().take(3).enumerate() {
        let number = field.parse::<u32>().map_err(|_| {
            format!("`{version}` is not a Windows Installer version: `{field}` is not a number")
        })?;
        let limit = if index == 2 { 65_535 } else { 255 };
        if number > limit {
            return Err(format!(
                "`{version}` is not a Windows Installer version: {number} is above {limit}"
            ));
        }
        numbers.push(number.to_string());
    }
    while numbers.len() < 3 {
        numbers.push("0".to_owned());
    }
    Ok(numbers.join("."))
}

/// Windows Installer recognizes an upgrade by this code, so it has to stay the
/// same across releases while differing between applications. Deriving it from
/// the bundle identifier gives both without a value to keep in a file.
fn upgrade_code(identifier: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"ice-bundle-upgrade-code:");
    hasher.update(identifier.as_bytes());
    let mut digest = hasher.finalize();
    digest[6] = (digest[6] & 0x0f) | 0x40;
    digest[8] = (digest[8] & 0x3f) | 0x80;

    let mut code = String::with_capacity(36);
    for (index, byte) in digest[..16].iter().enumerate() {
        if matches!(index, 4 | 6 | 8 | 10) {
            code.push('-');
        }
        write!(code, "{byte:02X}").expect("writing to a String cannot fail");
    }
    code
}

fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn sign(target: &Path) -> Result<(), String> {
    let Some(certificate) = setting("ICE_WINDOWS_CERTIFICATE") else {
        println!(
            "not signing `{}`; set ICE_WINDOWS_CERTIFICATE to sign",
            target.display()
        );
        return Ok(());
    };
    let mut arguments = vec![
        "sign".to_owned(),
        "/fd".to_owned(),
        "SHA256".to_owned(),
        // A countersigned timestamp is what keeps the signature valid after
        // the certificate itself expires.
        "/tr".to_owned(),
        setting("ICE_WINDOWS_TIMESTAMP_URL").unwrap_or_else(|| DEFAULT_TIMESTAMP_URL.to_owned()),
        "/td".to_owned(),
        "SHA256".to_owned(),
        "/f".to_owned(),
        certificate,
    ];
    if let Some(password) = setting("ICE_WINDOWS_CERTIFICATE_PASSWORD") {
        arguments.extend(["/p".to_owned(), password]);
    }
    arguments.push(path(target));
    tool("signtool", &arguments)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bundle::tests::showcase_meta;

    /// Resolving for Windows carries the `.exe` suffix into the executable
    /// name, and both the installed file and the shortcut depend on it.
    fn windows_meta() -> BundleMeta {
        BundleMeta {
            executable: "showcase.exe".into(),
            ..showcase_meta()
        }
    }

    #[test]
    fn the_package_installs_per_user_with_a_shortcut() {
        let authoring = package_authoring(
            &windows_meta(),
            Path::new("/build/showcase.exe"),
            Some(Path::new("/build/Showcase.ico")),
        )
        .expect("author the package");

        assert!(authoring.contains(r#"Scope="perUser""#), "{authoring}");
        assert!(authoring.contains(r#"Name="Showcase""#));
        assert!(authoring.contains(r#"Version="0.1.0""#));
        assert!(authoring.contains(r#"Source="/build/showcase.exe""#));
        assert!(
            authoring.contains(r#"Name="showcase.exe""#),
            "the installed name"
        );
        assert!(
            authoring.contains(r#"Target="[INSTALLFOLDER]showcase.exe""#),
            "the shortcut must point at the name the file is installed under"
        );
        assert!(authoring.contains(r#"<Icon Id="AppIcon" SourceFile="/build/Showcase.ico" />"#));
        assert!(authoring.contains(r#"<Property Id="ARPPRODUCTICON" Value="AppIcon" />"#));
        assert!(authoring.contains(r#"Icon="AppIcon""#), "the shortcut too");
        assert!(authoring.contains("<MajorUpgrade"), "upgrades replace");
        assert!(
            authoring.contains(&format!(
                r#"UpgradeCode="{}""#,
                upgrade_code("dev.ducktape.ui.showcase")
            )),
            "{authoring}"
        );
    }

    #[test]
    fn an_icon_free_package_names_no_icon() {
        let authoring = package_authoring(&windows_meta(), Path::new("/build/showcase.exe"), None)
            .expect("author the package");
        assert!(!authoring.contains("AppIcon"), "{authoring}");
    }

    #[test]
    fn authoring_escapes_what_would_break_the_xml() {
        let authoring = package_authoring(
            &BundleMeta {
                name: "Ice & Snow".into(),
                maintainer: "Bob <bob@example.com>".into(),
                ..windows_meta()
            },
            Path::new("/build/showcase.exe"),
            None,
        )
        .expect("author the package");
        assert!(
            authoring.contains(r#"Name="Ice &amp; Snow""#),
            "{authoring}"
        );
        assert!(
            authoring.contains(r#"Manufacturer="Bob""#),
            "the address is not part of the manufacturer name: {authoring}"
        );
    }

    #[test]
    fn the_upgrade_code_is_stable_per_application() {
        let code = upgrade_code("dev.ducktape.ui.showcase");
        assert_eq!(
            code,
            upgrade_code("dev.ducktape.ui.showcase"),
            "an upgrade depends on this never moving"
        );
        assert_ne!(code, upgrade_code("dev.ducktape.ui.trading"));
        assert_eq!(code.len(), 36, "{code}");
        let fields = code.split('-').map(str::len).collect::<Vec<_>>();
        assert_eq!(fields, [8, 4, 4, 4, 12]);
        assert!(
            code.chars().all(|c| c.is_ascii_hexdigit() || c == '-'),
            "{code}"
        );
        assert_eq!(&code[14..15], "4", "the version field");
        assert!(matches!(&code[19..20], "8" | "9" | "A" | "B"), "{code}");
    }

    #[test]
    #[ignore = "allocation contract; run alone with --test-threads=1"]
    fn performance_contract_upgrade_code_allocates_only_its_output() {
        const CODES: u64 = 100;

        let expected = upgrade_code("dev.ducktape.ui.showcase");
        let measured = crate::allocation::clean_window(CODES, || {
            for _ in 0..CODES {
                assert_eq!(upgrade_code("dev.ducktape.ui.showcase"), expected);
            }
        });

        assert_eq!(
            measured,
            (CODES, CODES * 36),
            "upgrade code allocations: {measured:?}"
        );
    }

    #[test]
    fn versions_windows_installer_cannot_compare_are_reported() {
        assert_eq!(product_version("0.1.0").expect("a release"), "0.1.0");
        assert_eq!(product_version("1.2").expect("two fields"), "1.2.0");
        assert_eq!(
            product_version("1.2.3.4").expect("a fourth field is ignored"),
            "1.2.3"
        );
        assert_eq!(
            product_version("1.2.3-rc.1").expect("a prerelease release"),
            "1.2.3"
        );
        // 256 wraps to 0 in the one-byte major field, so an upgrade would read
        // as a downgrade rather than fail.
        let error = product_version("256.0.0").expect_err("an unrepresentable major");
        assert!(error.contains("above 255"), "{error}");
        assert!(product_version("1.0.70000").is_err(), "the build field");
        assert!(product_version("nightly").is_err());
    }

    #[test]
    fn architectures_use_the_names_wix_knows() {
        assert_eq!(wix_architecture("x86_64").expect("x64"), "x64");
        assert_eq!(wix_architecture("aarch64").expect("arm64"), "arm64");
        assert!(wix_architecture("riscv64").is_err());
    }
}
