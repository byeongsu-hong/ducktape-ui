//! Quiescent state transfer into a fresh driver, without replaying app boot.
use super::*;

/// Generated Tree apps with a complete owned state codec.
/// Unsupported state reports an error instead of partially restoring an app.
pub trait SnapshotApp: App {
    fn snapshot(&self) -> Result<Vec<u8>, String>;
    fn restore(bytes: &[u8]) -> Result<Self, String>;
}

impl<A: SnapshotApp> Driver<A> {
    /// A live one-shot task may hold a user write. Do not retire it during reload.
    pub fn snapshot(&self) -> Result<Vec<u8>, String> {
        let _context = self.slots.enter();
        if slots::editor_pending()
            || self.busy
            || self.tasks.iter().any(|task| !task.subscription)
            || slots::has_deferred()
        {
            return Err("guest has pending work; snapshot after it settles".into());
        }
        self.app.snapshot()
    }

    /// Decodes into an independent context. Errors leave the caller's old driver
    /// untouched; callers replace it only after this returns a valid candidate.
    pub fn from_snapshot(bytes: &[u8], macos: bool) -> Result<Self, String> {
        let slots = slots::Context::with_macos(macos);
        let _context = slots.enter();
        let app = A::restore(bytes)?;
        let (subscribed, produced) = mpsc::channel(SUBSCRIPTION_QUEUE);
        Ok(Self {
            slots,
            window: iced::window::Id::unique(),
            app,
            tasks: Vec::new(),
            tracker: Tracker::new(),
            subscribed,
            produced,
            last_root: None,
            busy: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    thread_local! { static BOOTS: Cell<u32> = const { Cell::new(0) }; }
    struct Counter(u32);
    impl App for Counter {
        type Message = u32;
        fn boot() -> (Self, iced::Task<u32>) {
            BOOTS.set(BOOTS.get() + 1);
            (Self(7), iced::Task::none())
        }
        fn view(&self) -> wire::Node {
            slots::message(1u32);
            wire::Node::empty()
        }
        fn update(&mut self, value: u32) -> iced::Task<u32> {
            self.0 += value;
            iced::Task::none()
        }
        fn subscription(&self) -> iced::Subscription<u32> {
            iced::event::listen_with(|event, _, _| {
                matches!(event, iced::Event::Keyboard(_)).then_some(1)
            })
        }
    }
    impl SnapshotApp for Counter {
        fn snapshot(&self) -> Result<Vec<u8>, String> {
            Ok(self.0.to_le_bytes().to_vec())
        }
        fn restore(bytes: &[u8]) -> Result<Self, String> {
            if !keyboard::command(iced::keyboard::Modifiers::LOGO) {
                return Err("restore did not enter the host platform context".into());
            }
            Ok(Self(u32::from_le_bytes(
                bytes.try_into().map_err(|_| "invalid counter")?,
            )))
        }
    }

    #[test]
    fn restores_without_boot_and_keeps_failed_candidate_separate() {
        BOOTS.set(0);
        let mut original = Driver::<Counter>::with_macos(false);
        original.tick(vec![]);
        original.tick(vec![wire::Event::Message(0)]);
        let bytes = original.snapshot().unwrap();
        let mut restored = Driver::<Counter>::from_snapshot(&bytes, true).unwrap();
        assert_eq!(
            BOOTS.get(),
            1,
            "restoring must never replay boot side effects"
        );
        assert_eq!(restored.app.0, 8);
        assert!(
            restored.tick(vec![]).root.is_some(),
            "replacement sends a complete first tree"
        );
        restored.tick(vec![wire::Event::Message(0)]);
        assert_eq!(restored.app.0, 9, "replacement routes use the new context");
        assert_eq!(original.app.0, 8, "instances remain independent");
        assert!(Driver::<Counter>::from_snapshot(&[1], true).is_err());
        original.tick(vec![wire::Event::Message(0)]);
        assert_eq!(
            original.app.0, 9,
            "failed restoration leaves old routes usable"
        );
    }

    #[test]
    fn restores_active_keyboard_subscription_and_delivers_new_events() {
        let mut original = Driver::<Counter>::new();
        original.tick(vec![]);
        let bytes = original
            .snapshot()
            .expect("settled subscriptions must allow snapshot");
        let mut restored = Driver::<Counter>::from_snapshot(&bytes, true).unwrap();
        restored.tick(vec![]);
        restored.tick(vec![wire::Event::Keyboard {
            event: wire::keyboard::Event::Modifiers(wire::keyboard::Modifiers::default()),
            captured: false,
        }]);
        assert_eq!(
            restored.app.0, 8,
            "restored subscription receives keyboard events"
        );
        assert_eq!(original.app.0, 7, "old subscription remains independent");
        assert!(restored.snapshot().is_ok());
    }

    #[test]
    fn rejects_live_tasks_busy_frames_and_deferred_boot() {
        let mut driver = Driver::<Counter>::new();
        driver.busy = true;
        assert!(driver.snapshot().is_err());
        driver.busy = false;
        {
            let _context = driver.slots.enter();
            slots::defer(vec![3u32]);
        }
        assert!(driver.snapshot().is_err());
        driver.tick(vec![]);
        assert!(driver.snapshot().is_ok());
        spawn(
            &mut driver.tasks,
            iced::Task::perform(std::future::pending::<u32>(), |v| v),
        );
        assert!(
            driver.snapshot().is_err(),
            "pending writes cannot silently disappear"
        );
    }
}
