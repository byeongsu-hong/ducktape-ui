//! Platform-dependent modifier meaning comes from the host, not wasm's target OS.
use iced::keyboard::Modifiers;

pub fn command_modifiers() -> Modifiers {
    if crate::slots::macos() {
        Modifiers::LOGO
    } else {
        Modifiers::CTRL
    }
}
pub fn command(modifiers: Modifiers) -> bool {
    modifiers.contains(command_modifiers())
}
pub fn jump(modifiers: Modifiers) -> bool {
    if crate::slots::macos() {
        modifiers.alt()
    } else {
        modifiers.control()
    }
}
pub fn macos_command(modifiers: Modifiers) -> bool {
    crate::slots::macos() && modifiers.logo()
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Platform {
        boot_command: Modifiers,
    }
    impl crate::App for Platform {
        type Message = ();
        fn boot() -> (Self, iced::Task<()>) {
            (
                Self {
                    boot_command: command_modifiers(),
                },
                iced::Task::none(),
            )
        }
        fn update(&mut self, _: ()) -> iced::Task<()> {
            iced::Task::none()
        }
        fn subscription(&self) -> iced::Subscription<()> {
            iced::Subscription::none()
        }
        fn view(&self) -> crate::wire::Node {
            assert_eq!(
                self.boot_command,
                command_modifiers(),
                "host platform applies before boot"
            );
            let mac = self.boot_command == Modifiers::LOGO;
            assert_eq!(command(Modifiers::LOGO), mac);
            assert_eq!(command(Modifiers::CTRL), !mac);
            assert_eq!(jump(Modifiers::ALT), mac);
            assert_eq!(jump(Modifiers::CTRL), !mac);
            assert_eq!(macos_command(Modifiers::LOGO), mac);
            crate::wire::Node::empty()
        }
    }
    #[test]
    fn host_platform_applies_at_boot_and_is_scoped_to_each_driver() {
        let mut mac = crate::Driver::<Platform>::with_macos(true);
        let mut other = crate::Driver::<Platform>::with_macos(false);
        assert_eq!(mac.app.boot_command, Modifiers::LOGO);
        assert_eq!(other.app.boot_command, Modifiers::CTRL);
        mac.tick(vec![]);
        other.tick(vec![]);
        mac.tick(vec![]);
    }
    struct Counter {
        count: usize,
        limit: usize,
    }
    impl crate::App for Counter {
        type Message = ();
        fn boot() -> (Self, iced::Task<()>) {
            (
                Self {
                    count: 0,
                    limit: usize::MAX,
                },
                iced::Task::none(),
            )
        }
        fn update(&mut self, _: ()) -> iced::Task<()> {
            self.count += 1;
            iced::Task::none()
        }
        fn subscription(&self) -> iced::Subscription<()> {
            if self.count >= self.limit {
                return iced::Subscription::none();
            }
            iced::event::listen_with(|event, _, _| {
                matches!(event, iced::Event::Keyboard(_)).then_some(())
            })
        }
        fn view(&self) -> crate::wire::Node {
            crate::wire::Node::empty()
        }
    }
    fn press() -> crate::wire::Event {
        crate::wire::Event::Keyboard {
            event: crate::wire::keyboard::Event::Modifiers(
                crate::wire::keyboard::Modifiers::default(),
            ),
            captured: false,
        }
    }
    #[test]
    fn keyboard_bursts_drain_without_loss_and_reconcile_subscriptions_between_events() {
        let mut driver = crate::Driver::<Counter>::new();
        driver.tick(vec![]);
        driver.tick(vec![press(); 150]);
        assert_eq!(
            driver.app.count, 150,
            "keyboard batches must not overflow tracker channels"
        );
        let mut stopped = crate::Driver::<Counter>::new();
        stopped.app.limit = 1;
        stopped.tick(vec![]);
        stopped.tick(vec![press(); 3]);
        assert_eq!(
            stopped.app.count, 1,
            "earlier keys can remove the subscription"
        );
        assert_eq!(driver.app.count, 150, "events belong to one driver");
    }
}
