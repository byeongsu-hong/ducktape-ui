//! Native executable entry point. Stdout is reserved for framed host IPC;
//! applications must send diagnostics to stderr.
use crate::{Driver, SnapshotApp, wire};
use wire::native::{Request, Response, read_packet, write_packet};

/// Runs an explicitly built, trusted native guest. `--manifest` reads only the
/// static declaration: packaging does not initialize the application.
pub fn run<A: SnapshotApp>(manifest: &[u8]) -> Result<(), String> {
    use std::io::Write;
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args == ["--manifest"] {
        return std::io::stdout()
            .write_all(manifest)
            .map_err(|error| error.to_string());
    }
    if args != ["--ice-native"] {
        return Err("use --manifest to package or --ice-native for the app-store host".into());
    }
    serve::<A>(&mut std::io::stdin().lock(), &mut std::io::stdout().lock())
}

fn serve<A: SnapshotApp>(
    input: &mut impl std::io::Read,
    output: &mut impl std::io::Write,
) -> Result<(), String> {
    let mut driver: Option<Driver<A>> = None;
    loop {
        let request = wire::decode::<Request>(&read_packet(input)?)?;
        let result: Response = (|| match request {
            Request::Init { macos } => {
                if driver.is_some() {
                    return Err("already initialized".into());
                }
                driver = Some(Driver::with_macos(macos));
                Ok(Vec::new())
            }
            Request::Tick(bytes) => {
                let events = wire::decode(&bytes)?;
                let mut frame = driver.as_mut().ok_or("initialize first")?.tick(events);
                if frame.unchanged || !frame.patches.is_empty() {
                    frame.root = None;
                }
                Ok(wire::encode(&frame))
            }
            Request::Snapshot => driver.as_ref().ok_or("initialize first")?.snapshot(),
            Request::Restore { state, macos } => {
                let candidate = Driver::from_snapshot(&state, macos)?;
                driver = Some(candidate);
                Ok(Vec::new())
            }
        })();
        write_packet(output, &wire::encode(&result))?;
    }
}
