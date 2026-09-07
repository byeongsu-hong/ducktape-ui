//! Explicit resource files, copied beside the installed executable.
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub(super) struct Resource {
    pub source: PathBuf,
    pub destination: PathBuf,
}

pub(super) fn collect(
    root: &Path,
    declarations: &[String],
    executable: &str,
) -> Result<Vec<Resource>, String> {
    fn visit(source: &Path, destination: &Path, files: &mut Vec<Resource>) -> Result<(), String> {
        let metadata = fs::symlink_metadata(source).map_err(|error| {
            format!(
                "cannot read bundle resource `{}`: {error}",
                source.display()
            )
        })?;
        if metadata.is_file() {
            files.push(Resource {
                source: source.to_owned(),
                destination: destination.to_owned(),
            });
        } else if metadata.is_dir() {
            let mut children = fs::read_dir(source)
                .map_err(|error| {
                    format!(
                        "cannot read bundle resource directory `{}`: {error}",
                        source.display()
                    )
                })?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|error| error.to_string())?;
            children.sort_by_key(|entry| entry.file_name());
            for child in children {
                visit(&child.path(), &destination.join(child.file_name()), files)?;
            }
        } else {
            return Err(format!(
                "bundle resource `{}` must be a regular file or directory; symlinks and special files are not supported",
                source.display()
            ));
        }
        Ok(())
    }
    let mut names = BTreeSet::from([executable.to_lowercase()]);
    let mut files = Vec::new();
    for declaration in declarations {
        let mut source = root.to_owned();
        for component in Path::new(declaration).components() {
            match component {
                std::path::Component::Normal(_) | std::path::Component::ParentDir => {
                    source.push(component.as_os_str());
                    let metadata = fs::symlink_metadata(&source).map_err(|error| {
                        format!("cannot read bundle resource `{declaration}`: {error}")
                    })?;
                    if metadata.file_type().is_symlink() {
                        return Err(format!("bundle resource `{declaration}` contains symlinks"));
                    }
                }
                std::path::Component::CurDir => {}
                _ => {
                    return Err(format!(
                        "bundle resource `{declaration}` must be a relative path"
                    ));
                }
            }
        }
        let name = source
            .file_name()
            .and_then(|name| name.to_str())
            .filter(|name| !name.is_empty())
            .ok_or_else(|| {
                format!("bundle resource `{declaration}` needs a file or directory name")
            })?;
        if !names.insert(name.to_lowercase()) {
            return Err(format!(
                "bundle resource `{declaration}` collides with installed name `{name}`"
            ));
        }
        visit(&source, Path::new(name), &mut files)?;
    }
    let mut destinations = BTreeMap::new();
    for file in &files {
        for destination in file
            .destination
            .ancestors()
            .filter(|path| !path.as_os_str().is_empty())
        {
            let name = destination.to_str().ok_or_else(|| {
                format!(
                    "bundle resource `{}` needs a UTF-8 filename",
                    file.source.display()
                )
            })?;
            let entry = (destination.to_owned(), destination == file.destination);
            if let Some(previous) = destinations.insert(name.to_lowercase(), entry.clone())
                && (previous != entry || entry.1)
            {
                return Err(format!("bundle resources collide at `{name}`"));
            }
        }
    }
    Ok(files)
}

pub(super) fn install(files: &[Resource], directory: &Path) -> Result<(), String> {
    for file in files {
        let destination = directory.join(&file.destination);
        super::create_dir(
            destination
                .parent()
                .expect("resource destination has parent"),
        )?;
        super::install(&file.source, &destination)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nested_files_keep_the_declared_basename_and_exact_bytes() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().join("package");
        fs::create_dir_all(root.join("target/views/nested")).unwrap();
        fs::write(
            root.join("target/views/nested/module.wasm"),
            b"\0asm\x01\0\0\0",
        )
        .unwrap();
        let files = collect(&root, &["target/views".into()], "app").unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].destination, Path::new("views/nested/module.wasm"));
        let installed = directory.path().join("installed");
        install(&files, &installed).unwrap();
        assert_eq!(
            fs::read(installed.join("views/nested/module.wasm")).unwrap(),
            b"\0asm\x01\0\0\0"
        );
    }
    #[test]
    fn missing_and_colliding_resources_are_errors() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path();
        assert!(
            collect(root, &["missing".into()], "app")
                .unwrap_err()
                .contains("missing")
        );
        fs::write(root.join("App"), b"must not replace executable").unwrap();
        assert!(
            collect(root, &["App".into()], "app")
                .unwrap_err()
                .contains("collides")
        );
        fs::create_dir_all(root.join("one/views")).unwrap();
        fs::create_dir_all(root.join("two/views")).unwrap();
        assert!(
            collect(root, &["one/views".into(), "two/views".into()], "app")
                .unwrap_err()
                .contains("collides")
        );
    }
    #[cfg(target_os = "linux")]
    #[test]
    fn case_collisions_between_files_and_parent_directories_are_errors() {
        let directory = tempfile::tempdir().unwrap();
        fs::create_dir_all(directory.path().join("views/foo")).unwrap();
        fs::write(directory.path().join("views/Foo"), b"file").unwrap();
        fs::write(directory.path().join("views/foo/module.wasm"), b"wasm").unwrap();
        assert!(
            collect(directory.path(), &["views".into()], "app")
                .unwrap_err()
                .contains("collide")
        );
    }
    #[cfg(unix)]
    #[test]
    fn explicit_paths_cannot_traverse_symlinked_directories() {
        let directory = tempfile::tempdir().unwrap();
        fs::create_dir(directory.path().join("real")).unwrap();
        fs::write(directory.path().join("real/module.wasm"), b"wasm").unwrap();
        std::os::unix::fs::symlink("real", directory.path().join("views")).unwrap();
        assert!(
            collect(directory.path(), &["views/module.wasm".into()], "app")
                .unwrap_err()
                .contains("symlinks")
        );
    }
    #[cfg(unix)]
    #[test]
    fn symbolic_links_are_not_followed_into_resource_payloads() {
        let directory = tempfile::tempdir().unwrap();
        fs::create_dir(directory.path().join("views")).unwrap();
        std::os::unix::fs::symlink(".", directory.path().join("views/cycle")).unwrap();
        assert!(
            collect(directory.path(), &["views".into()], "app")
                .unwrap_err()
                .contains("symlinks")
        );
    }
}
