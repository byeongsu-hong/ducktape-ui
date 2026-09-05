use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::Duration;

pub(super) const READY_PATH_ENV: &str = "ICE_DEV_READY_PATH";
pub(super) const READY_TOKEN_ENV: &str = "ICE_DEV_READY_TOKEN";
/// Points a launched app at the template file this runner rewrites, so a view
/// edit reaches it without a rebuild.
pub(super) const TEMPLATE_PATH_ENV: &str = "ICE_TEMPLATE_PATH";

const RESTART_READY_TIMEOUT: Duration = Duration::from_secs(30);
static NEXT_EXECUTABLE_COPY: AtomicU64 = AtomicU64::new(0);
static NEXT_READY_TOKEN: AtomicU64 = AtomicU64::new(0);
static STOP_REQUESTED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub(super) fn install_stop_handler() -> Result<(), String> {
    STOP_REQUESTED.store(false, Ordering::Release);
    ctrlc::set_handler(|| STOP_REQUESTED.store(true, Ordering::Release))
        .map_err(|error| format!("ice dev: cannot install interrupt handler: {error}"))
}

pub(super) fn stop_requested() -> bool {
    STOP_REQUESTED.load(Ordering::Acquire)
}

pub(super) fn runtime_args(cargo_args: &[String]) -> &[String] {
    cargo_args
        .iter()
        .position(|arg| arg == "--")
        .and_then(|separator| cargo_args.get(separator + 1..))
        .unwrap_or_default()
}

pub(super) struct StagedExecutable {
    path: PathBuf,
}

impl StagedExecutable {
    pub(super) fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for StagedExecutable {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

pub(super) fn stage_executable(artifact: &Path, revision: u64) -> Result<StagedExecutable, String> {
    let extension = artifact.extension().map(OsString::from);
    let stem = artifact
        .file_stem()
        .or_else(|| artifact.file_name())
        .unwrap_or_else(|| std::ffi::OsStr::new("app"));
    loop {
        let copy_id = NEXT_EXECUTABLE_COPY.fetch_add(1, Ordering::Relaxed);
        let mut name = stem.to_os_string();
        name.push(format!(
            ".ice-dev-{}-r{revision}-{copy_id}",
            std::process::id()
        ));
        if let Some(extension) = &extension {
            name.push(".");
            name.push(extension);
        }
        let path = artifact.with_file_name(name);
        let mut destination = match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(destination) => destination,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error.to_string()),
        };
        let copy = (|| {
            let mut source = fs::File::open(artifact)?;
            std::io::copy(&mut source, &mut destination)?;
            destination.sync_all()?;
            fs::set_permissions(&path, source.metadata()?.permissions())
        })();
        if let Err(error) = copy {
            drop(destination);
            let _ = fs::remove_file(&path);
            return Err(format!(
                "ice dev: cannot stage executable {}: {error}",
                artifact.display()
            ));
        }
        drop(destination);
        return Ok(StagedExecutable { path });
    }
}

pub(super) struct ChildGuard {
    child: Child,
    executable: Option<StagedExecutable>,
}

impl ChildGuard {
    #[cfg(test)]
    pub(super) fn id(&self) -> u32 {
        self.child.id()
    }

    pub(super) fn spawn_owned(
        root: &Path,
        executable: StagedExecutable,
        args: &[String],
        template: Option<&Path>,
    ) -> Result<Self, String> {
        Self::spawn_owned_with_ready(root, executable, args, None, template)
    }

    #[cfg(test)]
    pub(super) fn spawn(root: &Path, executable: &Path, args: &[String]) -> Result<Self, String> {
        Self::spawn_process(root, executable, args, None, None, None)
    }

    #[cfg(test)]
    pub(super) fn spawn_with_ready(
        root: &Path,
        executable: &Path,
        args: &[String],
        ready: &Path,
        token: &str,
    ) -> Result<Self, String> {
        Self::spawn_process(root, executable, args, Some((ready, token)), None, None)
    }

    fn spawn_owned_with_ready(
        root: &Path,
        executable: StagedExecutable,
        args: &[String],
        ready: Option<(&Path, &str)>,
        template: Option<&Path>,
    ) -> Result<Self, String> {
        let path = executable.path().to_owned();
        Self::spawn_process(root, &path, args, ready, Some(executable), template)
    }

    fn spawn_process(
        root: &Path,
        executable: &Path,
        args: &[String],
        ready: Option<(&Path, &str)>,
        owned_executable: Option<StagedExecutable>,
        template: Option<&Path>,
    ) -> Result<Self, String> {
        let mut command = Command::new(executable);
        command
            .args(args)
            .env_remove(READY_PATH_ENV)
            .env_remove(READY_TOKEN_ENV)
            .env_remove(TEMPLATE_PATH_ENV)
            .current_dir(root);
        if let Some((ready, token)) = ready {
            command
                .env(READY_PATH_ENV, ready)
                .env(READY_TOKEN_ENV, token);
        }
        if let Some(template) = template {
            command.env(TEMPLATE_PATH_ENV, template);
        }
        let mut attempts = 0;
        let child = loop {
            match command.spawn() {
                Ok(child) => break child,
                Err(error) if executable_is_temporarily_busy(&error) && attempts < 20 => {
                    attempts += 1;
                    thread::sleep(Duration::from_millis(5));
                }
                Err(error) => return Err(error.to_string()),
            }
        };
        Ok(Self {
            child,
            executable: owned_executable,
        })
    }

    pub(super) fn try_wait(&mut self) -> Result<Option<std::process::ExitStatus>, String> {
        self.child.try_wait().map_err(|error| error.to_string())
    }

    pub(super) fn restart(
        &mut self,
        root: &Path,
        executable: StagedExecutable,
        args: &[String],
        ready_base: &Path,
        revision: u64,
        template: Option<&Path>,
    ) -> Result<(), String> {
        let token_id = NEXT_READY_TOKEN.fetch_add(1, Ordering::Relaxed);
        let token = format!("{}-{revision}-{token_id}", std::process::id());
        let ready = ready_base.with_extension(format!("revision-{revision}-{token_id}.ready"));
        remove_file_if_exists(&ready)?;
        let mut candidate =
            Self::spawn_owned_with_ready(root, executable, args, Some((&ready, &token)), template)?;
        if let Err(error) = candidate.wait_ready(&ready, &token) {
            drop(candidate);
            let _ = fs::remove_file(&ready);
            return Err(error);
        }
        if let Err(error) = self.terminate() {
            drop(candidate);
            let _ = fs::remove_file(&ready);
            return Err(error);
        }
        *self = candidate;
        let _ = fs::remove_file(ready);
        Ok(())
    }

    pub(super) fn wait_ready(&mut self, ready: &Path, token: &str) -> Result<(), String> {
        self.wait_ready_with_timeout(ready, token, RESTART_READY_TIMEOUT)
    }

    pub(super) fn wait_ready_with_timeout(
        &mut self,
        ready: &Path,
        token: &str,
        timeout: Duration,
    ) -> Result<(), String> {
        let started = std::time::Instant::now();
        loop {
            if stop_requested() {
                return Err("candidate startup interrupted".to_owned());
            }
            if let Some(status) = self.try_wait()? {
                return Err(format!(
                    "candidate exited with {status} before reporting readiness token `{token}`"
                ));
            }
            match fs::read_to_string(ready) {
                Ok(value) if value == token => {
                    if let Some(status) = self.try_wait()? {
                        return Err(format!(
                            "candidate exited with {status} after reporting readiness token `{token}`"
                        ));
                    }
                    return Ok(());
                }
                // A token is never empty, so an empty file is one that exists
                // but has not been written yet: a writer that creates and then
                // fills it is still on its way, not reporting nonsense.
                Ok(value) if value.is_empty() => {}
                Ok(value) => {
                    return Err(format!(
                        "candidate reported unexpected readiness token {value:?}; expected {token:?}"
                    ));
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.to_string()),
            }
            if started.elapsed() >= timeout {
                return Err(format!(
                    "candidate did not report readiness token {token:?} within {} milliseconds",
                    timeout.as_millis()
                ));
            }
            thread::sleep(Duration::from_millis(25));
        }
    }

    fn terminate(&mut self) -> Result<(), String> {
        if self.try_wait()?.is_some() {
            return Ok(());
        }
        self.child.kill().map_err(|error| error.to_string())?;
        self.child.wait().map_err(|error| error.to_string())?;
        Ok(())
    }
}

fn executable_is_temporarily_busy(error: &std::io::Error) -> bool {
    #[cfg(unix)]
    {
        error.raw_os_error() == Some(26)
    }
    #[cfg(not(unix))]
    {
        let _ = error;
        false
    }
}

fn remove_file_if_exists(path: &Path) -> Result<(), String> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.to_string()),
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        self.executable.take();
    }
}
