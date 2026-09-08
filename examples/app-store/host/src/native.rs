//! Trusted native Tree guests. Only this launch path executes a package; the
//! catalog scanner never does. IPC shares the wasm payloads and host policies,
//! but a native executable has the user's full operating-system permissions.
use crate::catalog::{CatalogEntry, native_hash, native_package};
use crate::limits::TICK_DEADLINE;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{Receiver, SyncSender, sync_channel};
use ui_lang_wire::native::{Request, Response, read_packet, write_packet};

pub(crate) struct Process {
    child: Child,
    requests: SyncSender<Vec<u8>>,
    responses: Receiver<Result<Response, String>>,
    _directory: Directory,
}

struct Directory(PathBuf);
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

impl Process {
    pub(crate) fn new(entry: &CatalogEntry) -> Result<Self, String> {
        Self::launch(
            entry,
            "--ice-native",
            native_package(std::path::Path::new(&entry.path))?,
        )
    }

    #[cfg(test)]
    pub(crate) fn new_authored(entry: &CatalogEntry) -> Result<Self, String> {
        Self::launch(
            entry,
            "--ice-authored-test",
            crate::catalog::native_package_with(
                std::path::Path::new(&entry.path),
                ui_lang_wire::authored::parse_manifest,
            )?,
        )
    }

    fn launch(
        entry: &CatalogEntry,
        argument: &str,
        (manifest, bytes): (Vec<u8>, Vec<u8>),
    ) -> Result<Self, String> {
        if native_hash(&manifest, &bytes) != entry.hash {
            return Err("native package changed since consent; Rescan, then Get it again".into());
        }
        // Launch precisely the bytes just verified, not a mutable catalog path.
        let mut random = [0u8; 16];
        getrandom::fill(&mut random).map_err(|error| error.to_string())?;
        let path =
            std::env::temp_dir().join(format!("ice-native-{:032x}", u128::from_le_bytes(random)));
        let mut builder = std::fs::DirBuilder::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        builder.create(&path).map_err(|error| error.to_string())?;
        let directory = Directory(path);
        let executable = directory
            .0
            .join(if cfg!(windows) { "app.exe" } else { "app" });
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o700);
        }
        let mut file = options
            .open(&executable)
            .map_err(|error| error.to_string())?;
        file.write_all(&bytes).map_err(|error| error.to_string())?;
        drop(file);
        let mut child = Command::new(executable)
            .arg(argument)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|error| error.to_string())?;
        let mut input = child.stdin.take().expect("piped stdin");
        let mut output = child.stdout.take().expect("piped stdout");
        let (requests, incoming) = sync_channel::<Vec<u8>>(1);
        let (outgoing, responses) = sync_channel(1);
        // Pipe writes can block too. Neither reading nor writing runs on the UI
        // thread; one bounded exchange is in flight and the caller owns kill.
        std::thread::spawn(move || {
            while let Ok(request) = incoming.recv() {
                let result = write_packet(&mut input, &request)
                    .and_then(|()| read_packet(&mut output))
                    .and_then(|bytes| ui_lang_wire::decode::<Response>(&bytes));
                let failed = result.is_err();
                if outgoing.send(result).is_err() || failed {
                    break;
                }
            }
        });
        Ok(Self {
            child,
            requests,
            responses,
            _directory: directory,
        })
    }

    pub(crate) fn call(&mut self, request: Request) -> Response {
        if ui_lang_wire::encoded_size(&request) > ui_lang_wire::native::MAX_PACKET_BYTES as u64 {
            return Err("native input exceeds the byte budget".into());
        }
        self.call_bytes(ui_lang_wire::encode(&request))
    }

    #[cfg(test)]
    pub(crate) fn call_authored(&mut self, request: ui_lang_wire::authored::Request) -> Response {
        if ui_lang_wire::encoded_size(&request) > ui_lang_wire::native::MAX_PACKET_BYTES as u64 {
            return Err("native input exceeds the byte budget".into());
        }
        self.call_bytes(ui_lang_wire::encode(&request))
    }

    fn call_bytes(&mut self, request: Vec<u8>) -> Response {
        self.requests
            .try_send(request)
            .map_err(|error| error.to_string())?;
        let result = self
            .responses
            .recv_timeout(TICK_DEADLINE)
            .map_err(|error| {
                format!(
                    "native guest exchange failed ({} ms deadline): {error}",
                    TICK_DEADLINE.as_millis()
                )
            })
            .and_then(|result| result);
        if result.is_err() {
            let _ = self.child.kill();
        }
        result.and_then(|response| response)
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        // _directory drops after this: Windows cannot remove a running executable.
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_catalog_reads_without_execution_and_binds_manifest_and_binary() {
        let mut random = [0; 16];
        getrandom::fill(&mut random).unwrap();
        let directory = Directory(std::env::temp_dir().join(format!(
            "ice-native-test-{:032x}",
            u128::from_le_bytes(random)
        )));
        let package = directory.0.join("probe.native");
        std::fs::create_dir_all(&package).unwrap();
        let manifest = b"ice.manifest.v1\nProbe\nCatalog test\n\n640.5,480.25";
        // Intentionally not executable code: scanning cannot launch it.
        std::fs::write(package.join("manifest"), manifest).unwrap();
        let executable = package.join(if cfg!(windows) { "app.exe" } else { "app" });
        std::fs::write(&executable, b"not executable").unwrap();
        let entries = crate::catalog::scan_dir(&directory.0);
        assert_eq!(
            entries.len(),
            1,
            "native package must be discoverable without executing it"
        );
        let entry = &entries[0];
        assert!(
            entry
                .capabilities
                .iter()
                .any(|cap| cap.name == "native-code"),
            "OS code execution requires explicit consent"
        );
        assert_eq!(entry.preferred_size.unwrap().dimensions(), [640.5, 480.25]);
        std::fs::write(
            package.join("manifest"),
            b"ice.manifest.v1\nProbe\nChanged consent\nclock,\nnone",
        )
        .unwrap();
        let changed = crate::catalog::scan_dir(&directory.0);
        assert_ne!(
            entry.hash, changed[0].hash,
            "manifest changes invalidate consent"
        );
        assert!(
            Process::new(entry)
                .err()
                .unwrap()
                .contains("changed since consent")
        );
        std::fs::write(&executable, b"different executable").unwrap();
        let rebuilt = crate::catalog::scan_dir(&directory.0);
        assert_ne!(
            changed[0].hash, rebuilt[0].hash,
            "binary changes invalidate consent"
        );
        assert!(
            Process::new(&changed[0])
                .err()
                .unwrap()
                .contains("changed since consent")
        );
    }
    #[test]
    #[ignore = "requires native Chaos package"]
    fn native_deadline_kills_the_process_and_drop_removes_verified_copy() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../target/app-store-native-catalog");
        let entry = crate::catalog::scan_dir(&path)
            .into_iter()
            .find(|entry| entry.name == "Chaos")
            .expect("build native packages first");
        let mut process = Process::new(&entry).unwrap();
        let directory = process._directory.0.clone();
        process.call(Request::Init { macos: false }).unwrap();
        let frame: ui_lang_wire::Frame = ui_lang_wire::decode(
            &process
                .call(Request::Tick(ui_lang_wire::encode(&Vec::<
                    ui_lang_wire::Event,
                >::new(
                ))))
                .unwrap(),
        )
        .unwrap();
        fn spin(node: &ui_lang_wire::Node) -> Option<u32> {
            if let ui_lang_wire::Node::Button { key, on_press, .. } = node
                && key.ends_with("/spin")
            {
                return *on_press;
            }
            node.children().iter().find_map(spin)
        }
        let route = spin(frame.root.as_ref().unwrap()).unwrap();
        assert!(
            process
                .call(Request::Tick(ui_lang_wire::encode(&vec![
                    ui_lang_wire::Event::Message(route)
                ])))
                .unwrap_err()
                .contains("deadline")
        );
        let mut ended = false;
        for _ in 0..50 {
            if process.child.try_wait().unwrap().is_some() {
                ended = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        assert!(
            ended,
            "deadline must kill the actual child, not just stop waiting for its response"
        );
        drop(process);
        assert!(
            !directory.exists(),
            "verified executable is removed after the child is reaped"
        );
    }
}
