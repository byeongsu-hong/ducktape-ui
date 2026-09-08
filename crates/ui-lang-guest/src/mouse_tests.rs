use crate::{App, Driver, wire};

struct MouseApp {
    events: Vec<(iced::mouse::Event, iced::event::Status)>,
    limit: usize,
}

impl App for MouseApp {
    type Message = (iced::mouse::Event, iced::event::Status);

    fn boot() -> (Self, iced::Task<Self::Message>) {
        (
            Self {
                events: Vec::new(),
                limit: usize::MAX,
            },
            iced::Task::none(),
        )
    }

    fn update(&mut self, event: Self::Message) -> iced::Task<Self::Message> {
        self.events.push(event);
        iced::Task::none()
    }

    fn subscription(&self) -> iced::Subscription<Self::Message> {
        if self.events.len() >= self.limit {
            iced::Subscription::none()
        } else {
            iced::event::listen_with(|event, status, _| match event {
                iced::Event::Mouse(event) => Some((event, status)),
                _ => None,
            })
        }
    }

    fn view(&self) -> wire::Node {
        wire::Node::empty()
    }
}

#[test]
fn mouse_delivery_preserves_metadata_and_reconciles_between_events() {
    use wire::mouse::{Button, Event, ScrollDelta};
    let events = [
        Event::CursorEntered,
        Event::CursorMoved { x: -12.5, y: 42.25 },
        Event::ButtonPressed(Button::Other(u16::MAX)),
        Event::WheelScrolled {
            delta: ScrollDelta::Pixels { x: 1.5, y: -3.25 },
        },
        Event::ButtonReleased(Button::Other(u16::MAX)),
        Event::CursorLeft,
    ];
    let batch: Vec<_> = events
        .into_iter()
        .enumerate()
        .map(|(index, event)| wire::Event::Mouse {
            event,
            captured: index % 2 == 0,
        })
        .collect();
    let mut driver = Driver::<MouseApp>::new();
    driver.tick(batch.clone());
    let expected: Vec<_> = events
        .into_iter()
        .enumerate()
        .map(|(index, event)| {
            (
                event.into(),
                if index % 2 == 0 {
                    iced::event::Status::Captured
                } else {
                    iced::event::Status::Ignored
                },
            )
        })
        .collect();
    assert_eq!(
        driver.app.events, expected,
        "native Mouse subscriptions must receive guest-local host events"
    );

    let mut stopped = Driver::<MouseApp>::new();
    stopped.app.limit = 1;
    stopped.tick(batch);
    assert_eq!(
        stopped.app.events.len(),
        1,
        "first mouse handler removes its subscription"
    );
    assert_eq!(
        driver.app.events, expected,
        "another driver cannot receive these events"
    );

    let mut burst = Driver::<MouseApp>::new();
    burst.tick(vec![
        wire::Event::Mouse {
            event: Event::ButtonPressed(Button::Left),
            captured: false
        };
        150
    ]);
    assert_eq!(
        burst.app.events.len(),
        150,
        "discrete mouse events must not overflow tracker channels"
    );
    burst.tick(vec![wire::Event::Mouse {
        event: Event::CursorMoved {
            x: f32::NAN,
            y: 0.0,
        },
        captured: false,
    }]);
    assert_eq!(
        burst.app.events.len(),
        150,
        "nonfinite positions cannot reach native listeners"
    );
}

#[test]
fn mouse_interest_follows_active_subscription_and_driver_scope() {
    struct Interested {
        enabled: bool,
    }
    impl App for Interested {
        type Message = ();
        fn boot() -> (Self, iced::Task<()>) {
            (Self { enabled: true }, iced::Task::none())
        }
        fn update(&mut self, _: ()) -> iced::Task<()> {
            self.enabled = false;
            iced::Task::none()
        }
        fn subscription(&self) -> iced::Subscription<()> {
            if self.enabled {
                crate::mouse::observe(iced::event::listen_with(|event, _, _| {
                    matches!(event, iced::Event::Mouse(_)).then_some(())
                }))
            } else {
                iced::Subscription::none()
            }
        }
        fn view(&self) -> wire::Node {
            wire::Node::empty()
        }
    }
    let mut first = Driver::<Interested>::new();
    let mut second = Driver::<Interested>::new();
    second.app.enabled = false;
    assert!(
        first.tick(vec![]).mouse_interest,
        "active listener requests host mouse delivery"
    );
    assert!(
        !second.tick(vec![]).mouse_interest,
        "interest belongs to one driver"
    );
    assert!(
        !first
            .tick(vec![wire::Event::Mouse {
                event: wire::mouse::Event::CursorEntered,
                captured: false
            }])
            .mouse_interest,
        "handler removes interest with its subscription"
    );
    first.app.enabled = true;
    assert!(
        first.tick(vec![]).mouse_interest,
        "reactivation restores interest"
    );
}
