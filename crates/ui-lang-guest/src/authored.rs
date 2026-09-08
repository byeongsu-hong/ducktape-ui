//! Explicit test-artifact control. No snapshot or serialized application state.
use crate::{App, Driver};
use iced::Task;

/// Implemented by the checked guest sidecar, never by a production export.
pub trait TestApp: App {
    const FINGERPRINT: u64;
    fn test_boot(test: u32) -> Result<(Self, Task<Self::Message>), String>;
    fn test_step(&self, test: u32, step: u32) -> Result<Option<Self::Message>, String>;
}

impl<A: TestApp> Driver<A> {
    /// Select a checked preset before initialization/mount, including its task.
    pub fn for_test(test: u32, macos: bool) -> Result<Self, String> {
        Self::initialize(macos, || A::test_boot(test))
    }

    /// Inspect live typed state or deliver a checked message through ordinary
    /// update/task settling. Pending host work does not prevent a predicate.
    pub fn test_step(&mut self, test: u32, step: u32) -> Result<(), String> {
        let _context = self.slots.enter();
        for message in crate::slots::take_deferred::<A::Message>() {
            crate::spawn(&mut self.tasks, self.app.update(message));
            self.settle();
        }
        self.settle();
        if let Some(message) = self.app.test_step(test, step)? {
            crate::spawn(&mut self.tasks, self.app.update(message));
            self.settle();
        }
        Ok(())
    }
}

/// Bounded test command handler used by both exported transports.
pub fn respond<A: TestApp + crate::SnapshotApp>(
    driver: &mut Option<Driver<A>>,
    request: crate::wire::authored::Request,
) -> Result<Vec<u8>, String> {
    use crate::wire::authored::Request;
    match request {
        Request::Begin {
            test,
            fingerprint,
            macos,
        } => {
            if fingerprint != A::FINGERPRINT {
                return Err("authored test artifact is stale".into());
            }
            if driver.is_some() {
                return Err("test must begin before initialization".into());
            }
            *driver = Some(Driver::for_test(test, macos)?);
            Ok(Vec::new())
        }
        Request::Step { test, step } => {
            driver
                .as_mut()
                .ok_or("begin authored test first")?
                .test_step(test, step)?;
            Ok(Vec::new())
        }
        Request::View(request) => {
            #[cfg(not(target_arch = "wasm32"))]
            {
                crate::native::respond(driver, request)
            }
            #[cfg(target_arch = "wasm32")]
            {
                let _ = request;
                Err("use the view exports for ordinary operations".into())
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn run<A: TestApp + crate::SnapshotApp>(manifest: &[u8]) -> Result<(), String> {
    use crate::wire::{
        decode, encode,
        native::{read_packet, write_packet},
    };
    use std::io::Write;
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args == ["--manifest"] {
        return std::io::stdout()
            .write_all(manifest)
            .map_err(|error| error.to_string());
    }
    if args != ["--ice-authored-test"] {
        return Err("this is an authored test artifact".into());
    }
    let mut driver = None;
    let mut input = std::io::stdin().lock();
    let mut output = std::io::stdout().lock();
    loop {
        let request = decode(&read_packet(&mut input)?)?;
        let response = respond::<A>(&mut driver, request);
        write_packet(&mut output, &encode(&response))?;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SnapshotApp;

    struct Counter {
        count: i64,
        // An owned value deliberately excluded from any snapshot representation.
        opaque: std::rc::Rc<()>,
    }
    impl App for Counter {
        type Message = i64;
        fn boot() -> (Self, Task<i64>) {
            (
                Self {
                    count: 0,
                    opaque: std::rc::Rc::new(()),
                },
                Task::none(),
            )
        }
        fn view(&self) -> crate::wire::Node {
            crate::wire::Node::empty()
        }
        fn update(&mut self, value: i64) -> Task<i64> {
            self.count = value;
            Task::none()
        }
        fn subscription(&self) -> iced::Subscription<i64> {
            iced::Subscription::none()
        }
    }
    impl TestApp for Counter {
        const FINGERPRINT: u64 = 42;
        fn test_boot(test: u32) -> Result<(Self, Task<i64>), String> {
            if test != 0 {
                return Err("unknown test".into());
            }
            let (app, _) = Self::boot();
            // A preset's initialization task is part of the same boot lifecycle.
            Ok((
                app,
                Task::done(7).chain(Task::future(std::future::pending())),
            ))
        }
        fn test_step(&self, test: u32, step: u32) -> Result<Option<i64>, String> {
            assert_eq!(test, 0);
            assert_eq!(std::rc::Rc::strong_count(&self.opaque), 1);
            match step {
                0 if self.count == 7 => Ok(None),
                1 => Ok(Some(self.count + 2)),
                2 if self.count == 9 => Ok(None),
                _ => Err("typed expectation failed".into()),
            }
        }
    }
    impl SnapshotApp for Counter {
        fn snapshot(&self) -> Result<Vec<u8>, String> {
            Err("opaque state".into())
        }
        fn restore(_: &[u8]) -> Result<Self, String> {
            Err("opaque state".into())
        }
    }

    #[test]
    fn preset_task_and_typed_dispatch_use_live_state_without_snapshot_quiescence() {
        let mut driver = Driver::<Counter>::for_test(0, false).unwrap();
        driver.test_step(0, 0).unwrap();
        assert!(driver.snapshot().unwrap_err().contains("pending"));
        driver.test_step(0, 1).unwrap();
        driver.test_step(0, 2).unwrap();
        assert_eq!(
            driver.test_step(0, 0).unwrap_err(),
            "typed expectation failed"
        );
    }
}
