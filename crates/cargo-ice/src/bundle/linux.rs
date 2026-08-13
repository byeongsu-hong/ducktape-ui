//! Debian packaging: the binary, a desktop entry, and the icon theme a
//! launcher reads, in one `.deb` that `apt install ./file.deb` accepts.

use super::{BundleMeta, capture, icon, path, tool};
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

pub(super) fn bundle(
    output: &Path,
    meta: &BundleMeta,
    executable: &Path,
    source: Option<&Path>,
    arch: &str,
) -> Result<Vec<PathBuf>, String> {
    let architecture = debian_architecture(arch)?;
    let root = output.join(format!("{}-{architecture}", meta.package));
    super::recreate(&root)?;

    let binaries = root.join("usr/bin");
    super::create_dir(&binaries)?;
    super::install(executable, &binaries.join(&meta.executable))?;

    let applications = root.join("usr/share/applications");
    super::create_dir(&applications)?;
    super::write(
        &applications.join(format!("{}.desktop", meta.identifier)),
        desktop_entry(meta).as_bytes(),
    )?;

    if let Some(source) = source {
        let svg = super::read(source)?;
        for size in icon::HICOLOR_SIZES {
            let directory = root.join(format!("usr/share/icons/hicolor/{size}x{size}/apps"));
            super::create_dir(&directory)?;
            super::write(
                &directory.join(format!("{}.png", meta.identifier)),
                &icon::png(&svg, *size)?,
            )?;
        }
        let scalable = root.join("usr/share/icons/hicolor/scalable/apps");
        super::create_dir(&scalable)?;
        super::write(&scalable.join(format!("{}.svg", meta.identifier)), &svg)?;
    }

    if let Some(copyright) = &meta.copyright {
        let documentation = root.join("usr/share/doc").join(&meta.package);
        super::create_dir(&documentation)?;
        super::write(
            &documentation.join("copyright"),
            format!("{copyright}\n").as_bytes(),
        )?;
    }

    let depends = shared_library_depends(&root, meta)?;
    let control = root.join("DEBIAN");
    super::create_dir(&control)?;
    super::write(
        &control.join("control"),
        control_file(meta, architecture, installed_size(&root)?, &depends).as_bytes(),
    )?;

    // Files copied out of a build tree carry the builder's umask, and Debian
    // rejects a group-writable payload. `X` keeps the execute bit exactly
    // where it already is: on the directories and the binary.
    tool(
        "chmod",
        &["-R".to_owned(), "u=rwX,go=rX".to_owned(), path(&root)],
    )?;

    let deb = output.join(format!(
        "{}_{}_{architecture}.deb",
        meta.package, meta.version
    ));
    tool(
        "dpkg-deb",
        &[
            "--build".to_owned(),
            "--root-owner-group".to_owned(),
            path(&root),
            path(&deb),
        ],
    )?;
    Ok(vec![deb])
}

fn debian_architecture(arch: &str) -> Result<&'static str, String> {
    match arch {
        "x86_64" => Ok("amd64"),
        "aarch64" => Ok("arm64"),
        other => Err(format!("no Debian architecture name for `{other}`")),
    }
}

/// The entry a launcher reads. Its file name is the application identifier,
/// which is also the Wayland app id iced reports, and that pairing is what
/// makes a running window show this icon instead of a placeholder.
fn desktop_entry(meta: &BundleMeta) -> String {
    format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Name={}\n\
         Comment={}\n\
         Exec={}\n\
         Icon={}\n\
         Terminal=false\n",
        meta.name, meta.description, meta.executable, meta.identifier
    )
}

fn control_file(
    meta: &BundleMeta,
    architecture: &str,
    installed_size: u64,
    depends: &str,
) -> String {
    let mut control = format!(
        "Package: {}\n\
         Version: {}\n\
         Architecture: {architecture}\n\
         Maintainer: {}\n\
         Installed-Size: {installed_size}\n\
         Section: utils\n\
         Priority: optional\n",
        meta.package, meta.version, meta.maintainer
    );
    if !depends.is_empty() {
        writeln!(&mut control, "Depends: {depends}").expect("writing to a String cannot fail");
    }
    if let Some(homepage) = &meta.homepage {
        writeln!(&mut control, "Homepage: {homepage}").expect("writing to a String cannot fail");
    }
    // dpkg reads everything up to the first newline as the synopsis, so it
    // stays one line; a description with newlines would end the field early.
    writeln!(
        &mut control,
        "Description: {}",
        meta.description.replace('\n', " ")
    )
    .expect("writing to a String cannot fail");
    control
}

/// Asks `dpkg-shlibdeps` what the built binary actually links, rather than
/// guessing a dependency list that drifts from the toolchain.
fn shared_library_depends(root: &Path, meta: &BundleMeta) -> Result<String, String> {
    // It insists on a source package layout, so it gets a minimal one inside
    // the staging tree, which is removed again before the tree is packaged.
    let debian = root.join("debian");
    super::create_dir(&debian)?;
    super::write(
        &debian.join("control"),
        format!(
            "Source: {package}\n\nPackage: {package}\nArchitecture: any\n",
            package = meta.package
        )
        .as_bytes(),
    )?;
    let reported = capture(
        "dpkg-shlibdeps",
        &["-O".to_owned(), format!("usr/bin/{}", meta.executable)],
        Some(root),
    );
    fs::remove_dir_all(&debian)
        .map_err(|error| format!("cannot clear `{}`: {error}", debian.display()))?;
    Ok(parse_shlibdeps(&reported?))
}

fn parse_shlibdeps(reported: &str) -> String {
    reported
        .lines()
        .find_map(|line| line.strip_prefix("shlibs:Depends="))
        .unwrap_or_default()
        .trim()
        .to_owned()
}

fn installed_size(root: &Path) -> Result<u64, String> {
    fn visit(path: &Path, total: &mut u64) -> Result<(), String> {
        for entry in fs::read_dir(path)
            .map_err(|error| format!("cannot read `{}`: {error}", path.display()))?
        {
            let entry = entry.map_err(|error| error.to_string())?;
            let file_type = entry.file_type().map_err(|error| error.to_string())?;
            if file_type.is_dir() {
                visit(&entry.path(), total)?;
            } else if file_type.is_file() {
                *total += entry.metadata().map_err(|error| error.to_string())?.len();
            }
        }
        Ok(())
    }

    let mut total = 0;
    visit(root, &mut total)?;
    // The field is in kibibytes, rounded up: dpkg reports it as disk usage.
    Ok(total.div_ceil(1024))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bundle::tests::showcase_meta;

    #[test]
    fn the_desktop_entry_names_the_identifier_a_launcher_matches() {
        let entry = desktop_entry(&showcase_meta());
        assert!(entry.starts_with("[Desktop Entry]\n"), "{entry}");
        for line in [
            "Type=Application",
            "Name=Showcase",
            "Exec=showcase",
            // The icon name and the file name are both the identifier, which
            // is what pairs a running window with this entry.
            "Icon=dev.ducktape.ui.showcase",
            "Terminal=false",
        ] {
            assert!(entry.lines().any(|entry| entry == line), "missing {line}");
        }
    }

    #[test]
    fn the_control_file_carries_every_field_dpkg_requires() {
        let control = control_file(&showcase_meta(), "amd64", 42, "libc6 (>= 2.34)");
        for line in [
            "Package: showcase",
            "Version: 0.1.0",
            "Architecture: amd64",
            "Installed-Size: 42",
            "Depends: libc6 (>= 2.34)",
            "Priority: optional",
        ] {
            assert!(control.lines().any(|field| field == line), "missing {line}");
        }
        assert!(
            control
                .lines()
                .any(|field| field.starts_with("Maintainer: "))
        );
        assert!(
            control
                .lines()
                .any(|field| field.starts_with("Description: "))
        );
    }

    #[test]
    #[ignore = "allocation contract; run alone with --test-threads=1"]
    fn performance_contract_control_file_writes_into_output() {
        const DOCUMENTS: usize = 1_024;
        const MAX_BLOCKS: u64 = 3_072;
        const MAX_BYTES: u64 = 662_528;

        let meta = showcase_meta();
        let expected = control_file(&meta, "amd64", 42, "libc6 (>= 2.34)").len();
        let _profiler = dhat::Profiler::builder().testing().build();
        for _ in 0..DOCUMENTS {
            std::hint::black_box(control_file(
                std::hint::black_box(&meta),
                "amd64",
                42,
                "libc6 (>= 2.34)",
            ));
        }
        let heap = dhat::HeapStats::get();

        eprintln!(
            "{DOCUMENTS} Debian control files ({expected} bytes): {} heap blocks / {} bytes",
            heap.total_blocks, heap.total_bytes
        );
        assert!(
            heap.total_blocks <= MAX_BLOCKS,
            "Debian control files allocated too many blocks: {heap:?}"
        );
        assert!(
            heap.total_bytes <= MAX_BYTES,
            "Debian control files allocated too many bytes: {heap:?}"
        );
    }

    #[test]
    fn a_description_stays_on_the_synopsis_line() {
        // A newline inside the value would end the field and turn the rest
        // into an unparsable control stanza.
        let control = control_file(
            &BundleMeta {
                description: "one\ntwo".into(),
                ..showcase_meta()
            },
            "amd64",
            1,
            "",
        );
        assert!(control.contains("Description: one two\n"), "{control}");
        assert!(!control.contains("Depends:"), "no depends, no empty field");
    }

    #[test]
    fn shlibdeps_output_becomes_the_depends_field() {
        assert_eq!(
            parse_shlibdeps("shlibs:Depends=libc6 (>= 2.34), libgcc-s1 (>= 3.0)\n"),
            "libc6 (>= 2.34), libgcc-s1 (>= 3.0)"
        );
        assert_eq!(parse_shlibdeps("dpkg-shlibdeps: warning: something\n"), "");
    }

    #[test]
    fn architectures_use_the_names_dpkg_knows() {
        assert_eq!(debian_architecture("x86_64").expect("amd64"), "amd64");
        assert_eq!(debian_architecture("aarch64").expect("arm64"), "arm64");
        assert!(debian_architecture("riscv64").is_err());
    }

    /// Drives the real `dpkg-shlibdeps` and `dpkg-deb`, so the staging layout
    /// and control stanza are wrong here rather than on a release tag.
    #[cfg(target_os = "linux")]
    #[test]
    fn a_staged_tree_becomes_an_installable_package() {
        let directory = tempfile::tempdir().expect("temporary directory");
        // A real ELF for this architecture is what dpkg-shlibdeps reads.
        let executable = directory.path().join("showcase");
        fs::copy(
            std::env::current_exe().expect("this test binary"),
            &executable,
        )
        .expect("stage an executable");
        let icon = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/icons/ice.svg");

        let built = bundle(
            directory.path(),
            &showcase_meta(),
            &executable,
            Some(&icon),
            "x86_64",
        )
        .expect("build the package");

        let [deb] = built.as_slice() else {
            panic!("one package, got {built:?}");
        };
        assert!(deb.is_file(), "{}", deb.display());
        let listed = capture("dpkg-deb", &["--contents".to_owned(), path(deb)], None)
            .expect("list the package");
        for entry in [
            "./usr/bin/showcase",
            "./usr/share/applications/dev.ducktape.ui.showcase.desktop",
            "./usr/share/icons/hicolor/256x256/apps/dev.ducktape.ui.showcase.png",
            "./usr/share/icons/hicolor/scalable/apps/dev.ducktape.ui.showcase.svg",
        ] {
            assert!(listed.contains(entry), "missing {entry} in:\n{listed}");
        }
        assert!(
            !listed.contains("/debian/"),
            "the shlibdeps scaffolding must not ship:\n{listed}"
        );
        // A group-writable payload is a Debian policy violation, and the
        // build tree's umask is where one would come from.
        for (mode, entry) in [
            ("-rwxr-xr-x", "./usr/bin/showcase"),
            (
                "-rw-r--r--",
                "./usr/share/applications/dev.ducktape.ui.showcase.desktop",
            ),
        ] {
            let line = listed
                .lines()
                .find(|line| line.ends_with(entry))
                .unwrap_or_else(|| panic!("{entry} is not listed in:\n{listed}"));
            assert!(line.starts_with(mode), "{entry} is {line}, wanted {mode}");
        }
        let fields = capture("dpkg-deb", &["--field".to_owned(), path(deb)], None)
            .expect("read the control fields");
        assert!(fields.contains("Package: showcase"), "{fields}");
        assert!(
            fields.contains("Depends: libc6"),
            "the linked libraries must be declared:\n{fields}"
        );
    }
}
