//! Keep peer forks from inheriting a writable native executable descriptor.
use std::io::{self, Write};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;

static LAUNCH: Mutex<()> = Mutex::new(());

pub(super) fn spawn(executable: &Path, bytes: &[u8], argument: &str) -> io::Result<Child> {
    // CLOEXEC closes a peer's inherited descriptor only at exec. Until then,
    // Linux refuses our executable with ETXTBSY even after our own writer closes.
    // Serialize only publication and spawn; guest IPC and waiting stay parallel.
    let _launch = LAUNCH.lock().expect("native launch lock poisoned");
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o700);
    }
    let mut file = options.open(executable)?;
    file.write_all(bytes)?;
    drop(file);
    Command::new(executable)
        .arg(argument)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::sync::{Arc, Barrier};

    #[test]
    fn concurrent_verified_copies_launch_without_writable_descriptor_races() {
        let bytes = Arc::new(std::fs::read("/bin/true").unwrap());
        let root = std::env::temp_dir().join(format!(
            "ice-native-launch-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
        ));
        std::fs::create_dir(&root).unwrap();
        let barrier = Arc::new(Barrier::new(16));
        let workers: Vec<_> = (0..16)
            .map(|worker| {
                let (bytes, root, barrier) = (bytes.clone(), root.clone(), barrier.clone());
                std::thread::spawn(move || {
                    let mut failures = Vec::new();
                    barrier.wait();
                    for iteration in 0..32 {
                        let path = root.join(format!("{worker}-{iteration}"));
                        match spawn(&path, &bytes, "--ice-native") {
                            Ok(mut child) => assert!(child.wait().unwrap().success()),
                            Err(error) => failures.push(error.to_string()),
                        }
                        std::fs::remove_file(path).unwrap();
                    }
                    failures
                })
            })
            .collect();
        let failures: Vec<_> = workers
            .into_iter()
            .flat_map(|worker| worker.join().unwrap())
            .collect();
        std::fs::remove_dir(root).unwrap();
        assert!(failures.is_empty(), "native launch failures: {failures:?}");
    }
}
