use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const READY_PATH_ENV: &str = "ICE_DEV_READY_PATH";
const READY_TOKEN_ENV: &str = "ICE_DEV_READY_TOKEN";
const RENDERER_ENV: &str = "ICED_BACKEND";
const TIMEOUT: Duration = Duration::from_secs(60);
const POLL_INTERVAL: Duration = Duration::from_millis(25);
const POST_DRAW_STABILITY: Duration = Duration::from_secs(1);

fn main() {
    if let Err(error) = run(env::args_os().skip(1)) {
        eprintln!("native WGPU smoke failed: {error}");
        std::process::exit(1);
    }
}

fn run(args: impl IntoIterator<Item = OsString>) -> Result<(), String> {
    let mut args = args.into_iter();
    let executable = args.next().ok_or_else(|| {
        "expected an application executable followed by optional arguments".to_owned()
    })?;
    let marker = ReadyMarker::new()?;
    let token = marker
        .path()
        .file_stem()
        .and_then(|stem| stem.to_str())
        .ok_or_else(|| "readiness marker token is not valid UTF-8".to_owned())?
        .to_owned();

    let mut child = ChildGuard::spawn(&executable, args, marker.path(), &token)?;
    let deadline = Instant::now() + TIMEOUT;

    loop {
        if let Some(status) = child.try_wait()? {
            return Err(format!(
                "{} exited with {status} before its first WGPU draw",
                Path::new(&executable).display()
            ));
        }

        if marker_matches(marker.path(), &token)? {
            child.require_stable_draw(&executable)?;
            child.terminate()?;
            println!(
                "{} completed its first WGPU draw",
                Path::new(&executable).display()
            );
            return Ok(());
        }

        if Instant::now() >= deadline {
            return Err(format!(
                "{} did not complete a WGPU draw within {} seconds",
                Path::new(&executable).display(),
                TIMEOUT.as_secs()
            ));
        }

        thread::sleep(POLL_INTERVAL);
    }
}

struct ReadyMarker {
    path: PathBuf,
}

impl ReadyMarker {
    fn new() -> Result<Self, String> {
        Ok(Self {
            path: unique_marker_path()?,
        })
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for ReadyMarker {
    fn drop(&mut self) {
        match fs::remove_file(&self.path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => eprintln!(
                "native WGPU smoke could not remove {}: {error}",
                self.path.display()
            ),
        }
    }
}

fn unique_marker_path() -> Result<PathBuf, String> {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("system clock is before the Unix epoch: {error}"))?
        .as_nanos();
    let directory = env::temp_dir();

    for suffix in 0..100 {
        let path = directory.join(format!(
            "ducktape-native-wgpu-{}-{nonce}-{suffix}.ready",
            std::process::id()
        ));
        if !path
            .try_exists()
            .map_err(|error| format!("cannot inspect {}: {error}", path.display()))?
        {
            return Ok(path);
        }
    }

    Err(format!(
        "cannot allocate a readiness marker below {}",
        directory.display()
    ))
}

fn marker_matches(path: &Path, token: &str) -> Result<bool, String> {
    match fs::read(path) {
        Ok(contents) if contents == token.as_bytes() => Ok(true),
        Ok(contents) => Err(format!(
            "readiness marker {} contained {:?}, expected {token:?}",
            path.display(),
            String::from_utf8_lossy(&contents)
        )),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(format!(
            "cannot read readiness marker {}: {error}",
            path.display()
        )),
    }
}

struct ChildGuard {
    child: Child,
}

impl ChildGuard {
    fn spawn(
        executable: &OsString,
        args: impl IntoIterator<Item = OsString>,
        marker: &Path,
        token: &str,
    ) -> Result<Self, String> {
        let child = Command::new(executable)
            .args(args)
            .env(RENDERER_ENV, "wgpu")
            .env(READY_PATH_ENV, marker)
            .env(READY_TOKEN_ENV, token)
            .spawn()
            .map_err(|error| {
                format!("cannot start {}: {error}", Path::new(executable).display())
            })?;
        Ok(Self { child })
    }

    fn try_wait(&mut self) -> Result<Option<std::process::ExitStatus>, String> {
        self.child
            .try_wait()
            .map_err(|error| format!("cannot inspect application process: {error}"))
    }

    fn terminate(&mut self) -> Result<(), String> {
        if self.try_wait()?.is_none() {
            self.child
                .kill()
                .map_err(|error| format!("cannot stop application process: {error}"))?;
            self.child
                .wait()
                .map_err(|error| format!("cannot reap application process: {error}"))?;
        }
        Ok(())
    }

    fn require_stable_draw(&mut self, executable: &OsString) -> Result<(), String> {
        let deadline = Instant::now() + POST_DRAW_STABILITY;
        while Instant::now() < deadline {
            if let Some(status) = self.try_wait()? {
                return Err(format!(
                    "{} exited with {status} immediately after its first WGPU draw",
                    Path::new(executable).display()
                ));
            }
            thread::sleep(POLL_INTERVAL);
        }
        Ok(())
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{marker_matches, unique_marker_path};
    use std::fs;

    #[test]
    fn missing_marker_is_not_ready() {
        let path = unique_marker_path().unwrap();
        assert!(!marker_matches(&path, "expected").unwrap());
    }

    #[test]
    fn exact_marker_is_ready() {
        let path = unique_marker_path().unwrap();
        fs::write(&path, "expected").unwrap();

        assert!(marker_matches(&path, "expected").unwrap());

        fs::remove_file(path).unwrap();
    }

    #[test]
    fn malformed_marker_is_rejected() {
        let path = unique_marker_path().unwrap();
        fs::write(&path, "wrong").unwrap();

        let error = marker_matches(&path, "expected").unwrap_err();

        fs::remove_file(path).unwrap();
        assert!(error.contains("contained \"wrong\", expected \"expected\""));
    }
}
